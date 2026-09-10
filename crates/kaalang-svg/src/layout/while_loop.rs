//! Routes the return to the entry junction on the line before its condition.

use kaalang_model::SemanticModel;

use super::{Connection, LANE, Point, Scene, route, vertical_gap};
use crate::topology::{Destination, NodeId, Source};

pub(super) fn route(scene: &mut Scene, model: &SemanticModel) -> Result<(), String> {
    let gap = vertical_gap(&scene.topology);
    for loop_index in (0..scene.topology.loops.len()).rev() {
        let loop_ = &scene.topology.loops[loop_index];
        let (header, entry, tail) = (loop_.header, loop_.entry, loop_.tail);
        let source = Source::Junction(tail);
        let destination = Destination::Junction(entry);
        let incoming = scene
            .connections
            .iter()
            .enumerate()
            .filter_map(|(index, edge)| {
                (edge.destination == Destination::Junction(tail)).then_some(index)
            })
            .collect::<Vec<_>>();
        let arrival = *incoming.first().expect("a repeating body reaches its tail");
        let points = scene.connections[arrival].points.clone();
        let start = *points.last().expect("an arrival has a route");
        let raised = (incoming.len() == 1)
            .then(|| compact_arrival(&points, gap))
            .flatten();
        let end = *scene
            .connections
            .iter()
            .find(|edge| edge.source == Source::Junction(entry))
            .and_then(|edge| edge.points.first())
            .expect("the loop entry leads to its condition");
        let body_end = model.flow.blocks[header]
            .loop_end
            .expect("a while owns a body");
        let body_nodes = scene.nodes.iter().filter(|node| {
            matches!(node.id, NodeId::Block(index) if (header..body_end).contains(&index))
                || matches!(node.id, NodeId::Case { choice, .. } if (header..body_end).contains(&choice))
        }).collect::<Vec<_>>();
        // A tail can occupy a branch column beyond every body node.
        let left = body_nodes
            .iter()
            .map(|node| Scene::bounds(node).0)
            .min()
            .unwrap_or(end.x)
            .min(start.x);
        let right = body_nodes
            .iter()
            .map(|node| Scene::bounds(node).2)
            .max()
            .unwrap_or(end.x)
            .max(start.x);
        let prefer_left = model.flow.blocks[header].yes_branch() == 0;
        let mut routed = false;
        // ponytail: bounded contour search; add placement feedback if a real
        // flow needs more than a clear lane on either side of its body.
        for points in raised.into_iter().chain(std::iter::once(points)) {
            let from = *points.last().expect("an arrival has a route");
            scene.connections[arrival].points = points;
            for offset in 1..=scene.topology.connections.len() + 2 {
                for use_left in [prefer_left, !prefer_left] {
                    let aside = if use_left {
                        left - offset as i32 * LANE
                    } else {
                        right + offset as i32 * LANE
                    };
                    let points = vec![
                        from,
                        Point {
                            x: aside,
                            y: from.y,
                        },
                        Point { x: aside, y: end.y },
                        end,
                    ];
                    scene.connections.push(Connection {
                        source,
                        destination,
                        points,
                    });
                    if route::verify(scene).is_none() {
                        routed = true;
                        break;
                    }
                    scene.connections.pop();
                }
                if routed {
                    break;
                }
            }
            if routed {
                break;
            }
        }
        if !routed {
            return Err(format!(
                "cannot route the return to while `{}` without a crossing",
                model.flow.blocks[header]
                    .description
                    .as_deref()
                    .unwrap_or_default()
            ));
        }
    }
    Ok(())
}

/// A sole arrival can turn at its last bend, or after the usual gap below a
/// vertical exit. The tail's placement row may be much lower.
fn compact_arrival(points: &[Point], gap: i32) -> Option<Vec<Point>> {
    let [.., bend, end] = points else {
        return None;
    };
    let y = bend.y + if points.len() == 2 { gap } else { 0 };
    if bend.x != end.x || y >= end.y {
        return None;
    }
    let mut compact = points[..points.len() - 1].to_vec();
    if y != bend.y {
        compact.push(Point { x: end.x, y });
    }
    Some(compact)
}
