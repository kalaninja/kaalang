//! Fits an entry and its first body node into space above a sibling row.

use std::collections::BTreeSet;

use kaalang_model::topology::{Destination, Source, Vertex};

use super::{Scene, conforms, vertical_gap};

pub(super) fn compact_entries(scene: &mut Scene) {
    let gap = vertical_gap(scene);
    for index in (0..scene.topology.loops.len()).rev() {
        let entry = scene.topology.loops[index].entry;
        let outgoing = scene
            .topology
            .outgoing(Vertex::Junction(entry))
            .collect::<Vec<_>>();
        let [edge] = outgoing.as_slice() else {
            continue;
        };
        let Destination::Node(node) = edge.destination else {
            continue;
        };
        if scene.topology.incoming(Vertex::Node(node)).count() != 1 {
            continue;
        }
        let arrivals = scene
            .topology
            .incoming(Vertex::Junction(entry))
            .collect::<Vec<_>>();
        let [initial] = arrivals.as_slice() else {
            continue;
        };
        let initial = scene
            .connections
            .iter()
            .find(|edge| edge.source == initial.source && edge.destination == initial.destination)
            .expect("the entry has its initial arrival");
        let [from, old_entry] = initial.points.as_slice() else {
            continue;
        };
        if from.x != old_entry.x {
            continue;
        }
        let (from, old_entry) = (*from, *old_entry);
        let body = scene.node(node);
        let (old_y, half) = (body.y, body.height / 2);
        let rows = scene
            .nodes
            .iter()
            .map(|node| node.y)
            .filter(|&y| y < old_y)
            .collect::<BTreeSet<_>>();
        for y in rows {
            let entry_y = y - half - gap;
            if entry_y < from.y + gap || entry_y > old_entry.y {
                continue;
            }
            let mut candidate = scene.clone();
            candidate
                .nodes
                .iter_mut()
                .find(|placed| placed.id == node)
                .expect("the first body node is placed")
                .y = y;
            for edge in &mut candidate.connections {
                if Vertex::from(edge.source) == Vertex::Node(node) {
                    let start = edge.points[0].y;
                    for point in edge.points.iter_mut().take_while(|point| point.y == start) {
                        point.y += y - old_y;
                    }
                }
                if edge.destination == Vertex::Node(node) {
                    edge.points
                        .last_mut()
                        .expect("a body arrival has an endpoint")
                        .y += y - old_y;
                }
                if edge.destination == Vertex::Junction(entry) {
                    for point in edge
                        .points
                        .iter_mut()
                        .rev()
                        .take_while(|point| point.y == old_entry.y)
                    {
                        point.y = entry_y;
                    }
                }
                if edge.source == Source::Junction(entry) {
                    edge.points[0].y = entry_y;
                }
            }
            if conforms(&mut candidate) {
                *scene = candidate;
                break;
            }
        }
    }
}
