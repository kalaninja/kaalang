//! Adjusts terminal routes to leave consistent clearance below loop returns.

use super::{Scene, route, vertical_gap};
use crate::topology::{Destination, NodeKind, Source};

const RETURN_GAP: i32 = 91;

pub(super) fn adjust(scene: &mut Scene) {
    if scene.topology.loops.is_empty() {
        return;
    }
    let end = scene
        .topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)
        .expect("a flow has an end")
        .id;
    let incoming = scene
        .topology
        .incoming(Destination::Node(end))
        .collect::<Vec<_>>();
    let terminal = match incoming.as_slice() {
        [edge] => match edge.source {
            Source::Junction(junction)
                if scene.topology.junctions[junction].wires == ["result"]
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
    let below_nodes = scene
        .nodes
        .iter()
        .filter(|node| node.id != end)
        .map(|node| Scene::bounds(node).3)
        .max()
        .unwrap_or(0)
        + vertical_gap(&scene.topology);
    let below_returns = scene
        .connections
        .iter()
        .filter(|edge| scene.is_back_edge(edge))
        .flat_map(|edge| &edge.points)
        .map(|point| point.y)
        .max()
        .unwrap_or(0)
        + RETURN_GAP;
    let delta = old_y - below_nodes.max(below_returns);
    if delta == 0 {
        return;
    }

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
    let end_index = scene
        .nodes
        .iter()
        .position(|node| node.id == end)
        .expect("end is placed");
    scene.nodes[end_index].y -= delta;
    if route::verify(scene).is_some() {
        scene.nodes[end_index].y += delta;
        for (index, points) in saved {
            scene.connections[index].points = points;
        }
    }
}
