//! Keeps end below the rest of the diagram, including loop returns.

use std::collections::BTreeSet;

use super::{Scene, route, vertical_gap};
use kaalang_model::topology::{Destination, NodeKind, Source};

pub(super) fn adjust(scene: &mut Scene) {
    if scene.topology.loops.is_empty() {
        return;
    }
    let Some(end) = scene
        .topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)
        .map(|node| node.id)
    else {
        return;
    };
    let incoming = scene
        .topology
        .incoming(Destination::Node(end))
        .collect::<Vec<_>>();
    let terminal = match incoming.as_slice() {
        [edge] => match edge.source {
            Source::Junction(junction)
                if scene.topology.end_merge == Some(junction)
                    && scene
                        .topology
                        .outgoing(Destination::Junction(junction))
                        .all(|edge| edge.destination == Destination::Node(end)) =>
            {
                Destination::Junction(junction)
            }
            _ => Destination::Node(end),
        },
        _ => Destination::Node(end),
    };
    let old_y = scene
        .connections
        .iter()
        .find(|edge| edge.destination == terminal)
        .and_then(|edge| edge.points.last())
        .expect("the terminal route has an arrival")
        .y;
    let gap = vertical_gap(scene);
    let below_nodes = scene
        .nodes
        .iter()
        .filter(|node| node.id != end)
        .map(|node| Scene::bounds(node).3)
        .max()
        .unwrap_or(0)
        + gap;
    let below_returns = scene
        .connections
        .iter()
        .filter(|edge| scene.is_back_edge(edge))
        .flat_map(|edge| &edge.points)
        .map(|point| point.y)
        .max()
        .unwrap_or(0)
        + gap;
    let end_index = scene
        .nodes
        .iter()
        .position(|node| node.id == end)
        .expect("end is placed");
    for y in BTreeSet::from([below_nodes, below_nodes.max(below_returns)]) {
        let delta = old_y - y;
        let mut saved = Vec::new();
        for (index, edge) in scene.connections.iter_mut().enumerate() {
            let from_merge = matches!(terminal, Destination::Junction(junction)
                if edge.source == Source::Junction(junction));
            if from_merge || edge.destination == terminal {
                saved.push((index, edge.points.clone()));
                for (position, point) in edge.points.iter_mut().enumerate() {
                    if from_merge || (position > 0 && point.y == old_y) {
                        point.y -= delta;
                    }
                }
            }
        }
        scene.nodes[end_index].y -= delta;
        if clears_returns(scene, terminal, y, gap) && route::verify(scene).is_none() {
            return;
        }
        scene.nodes[end_index].y += delta;
        for (index, points) in saved {
            scene.connections[index].points = points;
        }
    }
}

/// Parallel terminal and return rails need the usual gap wherever their
/// horizontal spans overlap, even when their centre lines do not intersect.
fn clears_returns(scene: &Scene, terminal: Destination, y: i32, gap: i32) -> bool {
    scene
        .connections
        .iter()
        .filter(|edge| edge.destination == terminal)
        .flat_map(|edge| edge.points.windows(2))
        .filter(|rail| rail[0].y == y && rail[1].y == y)
        .all(|rail| {
            scene
                .connections
                .iter()
                .filter(|edge| scene.is_back_edge(edge))
                .flat_map(|edge| edge.points.windows(2))
                .all(|segment| {
                    segment[0].y != segment[1].y
                        || (segment[0].y - y).abs() >= gap
                        || rail[0].x.min(rail[1].x) > segment[0].x.max(segment[1].x)
                        || rail[0].x.max(rail[1].x) < segment[0].x.min(segment[1].x)
                })
        })
}

/// Compaction may remove space, but nothing may end below the final block.
pub(super) fn verify(scene: &Scene) -> Option<String> {
    let end = scene
        .topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)?;
    let top = Scene::bounds(scene.node(end.id)).1;
    let misplaced_node = scene
        .nodes
        .iter()
        .any(|node| node.id != end.id && Scene::bounds(node).3 >= top);
    // A return beside end may align with its top edge; the block itself still
    // sits below it. No connection may descend alongside the block's body.
    let misplaced_route = scene
        .connections
        .iter()
        .any(|edge| edge.points.iter().any(|point| point.y > top));
    (misplaced_node || misplaced_route)
        .then(|| "end must be below every other node and loop return".to_owned())
}
