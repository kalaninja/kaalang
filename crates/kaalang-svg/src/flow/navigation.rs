//! The hotspots of a drawn flow: its nodes, its cycle boundaries, the wire
//! names in its labels, each with the source it stands for, and its
//! connections and merges, each with its ribbon.

use std::collections::BTreeMap;

use kaalang_compiler::{
    Analysis, ProducerId, SemanticModel,
    topology::{Node, NodeId, NodeKind, Source, Vertex},
};
use proc_macro2::Span;

use super::layout::{LabelKind, Point, Scene, name_rects};
use crate::{Hotspot, HotspotKind, RibbonWire};

/// How far a connection's strip reaches either side of its line, and a merge's
/// square from its centre: a pointer target 24 px across at the diagram's own
/// size. Neighbouring strips overlap, as horizontal routes may run 20 apart.
const REACH: i32 = 12;

/// Appends the hotspots of one scene drawn at `origin` on the canvas: the local
/// flow `part` of `root`, as `Analysis::node_span` takes them.
pub(crate) fn collect(
    scene: &Scene,
    root: &SemanticModel,
    part: Option<usize>,
    origin: Point,
    hotspots: &mut Vec<Hotspot>,
) {
    let mut push = |kind, (left, top, right, bottom): (i32, i32, i32, i32), span: Option<Span>| {
        hotspots.push(Hotspot {
            kind,
            left: origin.x + left,
            top: origin.y + top,
            right: origin.x + right,
            bottom: origin.y + bottom,
            span,
        });
    };
    let analysis = &root.analysis;
    let model = part.map_or(root, |stage| &root.stages[stage]);
    let flow = &model.analysis.flow;
    for node in &scene.nodes {
        let drawn = scene.topology.node(node.id);
        let Some(span) = analysis.node_span(part, drawn) else {
            continue;
        };
        let kind = match (drawn.kind, node.id) {
            (NodeKind::Call, NodeId::Block(block)) => HotspotKind::Call {
                callee: flow.blocks[block].callee_name().span(),
            },
            _ => HotspotKind::Node,
        };
        push(kind, Scene::bounds(node), Some(span));
    }
    let boundaries = scene.topology.cycle_boundaries.iter();
    for (boundary, region) in boundaries.zip(&scene.cycle_regions) {
        let cycle = Node {
            id: NodeId::Block(boundary.header),
            kind: NodeKind::Cycle,
        };
        if region.drawn
            && let Some(span) = analysis.node_span(part, &cycle)
        {
            push(HotspotKind::Node, region.bounds(), Some(span));
        }
    }
    for label in &scene.labels {
        let LabelKind::Wire(list) = label.kind else {
            continue;
        };
        let bindings = scene.captions.bindings(list);
        for (position, rect) in name_rects(label, scene.captions.names(list)) {
            if let Some(binding) = bindings[position] {
                push(HotspotKind::Wire, rect, Some(binding));
            }
        }
    }

    push_ribbons(scene, model, analysis, part, &mut push);
}

/// Hands `push` a strip along each segment of each drawn connection and a square
/// on each merge, each with its ribbon: the local flow `model` of the flow
/// `root` analyzes.
fn push_ribbons(
    scene: &Scene,
    model: &SemanticModel,
    root: &Analysis,
    part: Option<usize>,
    push: &mut impl FnMut(HotspotKind, (i32, i32, i32, i32), Option<Span>),
) {
    let flow = &model.analysis.flow;
    let ribbons = model.ribbons();
    // Each wire is named and bound once, however many ribbons list it.
    let mut named = BTreeMap::<ProducerId, RibbonWire>::new();
    let mut ribbon = |producers: &[ProducerId]| -> Vec<RibbonWire> {
        producers
            .iter()
            .map(|&producer| {
                named
                    .entry(producer)
                    .or_insert_with(|| RibbonWire {
                        name: flow.wire_name(flow.wire(producer)),
                        binding: root.binding(part, producer),
                    })
                    .clone()
            })
            .collect()
    };
    let around = |from: Point, to: Point| {
        (
            from.x.min(to.x) - REACH,
            from.y.min(to.y) - REACH,
            from.x.max(to.x) + REACH,
            from.y.max(to.y) + REACH,
        )
    };
    // The ribbon of each drawn connection, by its two ends, which no two share.
    let by_ends: BTreeMap<_, _> = model
        .topology
        .connections
        .iter()
        .zip(&ribbons.connections)
        .chain(model.topology.back_edges.iter().zip(&ribbons.back_edges))
        .map(|(edge, producers)| ((edge.source, edge.destination), producers.as_slice()))
        .collect();
    for connection in &scene.connections {
        let Some(&producers) = by_ends.get(&(connection.source, connection.destination)) else {
            continue;
        };
        let ribbon = ribbon(producers);
        // A route may repeat a point where it bends, which draws nothing.
        let mut points = connection.points.clone();
        points.dedup();
        let last = points.len().saturating_sub(2);
        for (index, segment) in points.windows(2).enumerate() {
            let (from, to) = (segment[0], segment[1]);
            // A strip reaches past a corner or a junction, but stops where its
            // connection meets a node.
            let mut bounds = around(from, to);
            if index == 0 && matches!(connection.source, Source::Exit(_)) {
                stop(&mut bounds, from, to);
            }
            if index == last && matches!(connection.destination, Vertex::Node(_)) {
                stop(&mut bounds, to, from);
            }
            push(
                HotspotKind::Connection {
                    ribbon: ribbon.clone(),
                },
                bounds,
                None,
            );
        }
    }
    let junctions = model.topology.junctions.iter().zip(&ribbons.junctions);
    for (junction, (merged, producers)) in junctions.enumerate() {
        if merged.merges.is_empty() {
            continue;
        }
        let Some(point) = scene.junction_at(junction) else {
            continue;
        };
        push(
            HotspotKind::Merge {
                ribbon: ribbon(producers),
            },
            around(point, point),
            None,
        );
    }
}

/// Moves the side of a segment's strip beyond `end` back to `end`, the other
/// end being `other`.
fn stop(bounds: &mut (i32, i32, i32, i32), end: Point, other: Point) {
    if end.x < other.x {
        bounds.0 = end.x;
    } else if end.x > other.x {
        bounds.2 = end.x;
    } else if end.y < other.y {
        bounds.1 = end.y;
    } else {
        bounds.3 = end.y;
    }
}
