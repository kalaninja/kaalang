//! The hotspots of a drawn flow: its nodes, its cycle boundaries, and the wire
//! names in its labels, each with the source it stands for.

use kaalang_compiler::{
    Analysis,
    topology::{Node, NodeId, NodeKind},
};
use proc_macro2::Span;

use super::layout::{LabelKind, Point, Scene, name_rects};
use crate::{Hotspot, HotspotKind};

/// Appends the hotspots of one scene drawn at `origin` on the canvas: the local
/// flow `part` of the flow `root` analyzes, as `Analysis::node_span` takes them.
pub(crate) fn collect(
    scene: &Scene,
    root: &Analysis,
    part: Option<usize>,
    origin: Point,
    hotspots: &mut Vec<Hotspot>,
) {
    let mut push = |kind, (left, top, right, bottom): (i32, i32, i32, i32), span: Span| {
        hotspots.push(Hotspot {
            kind,
            left: origin.x + left,
            top: origin.y + top,
            right: origin.x + right,
            bottom: origin.y + bottom,
            span,
        });
    };
    let flow = &part.map_or(root, |stage| &root.stages[stage].analysis).flow;
    for node in &scene.nodes {
        let drawn = scene.topology.node(node.id);
        let Some(span) = root.node_span(part, drawn) else {
            continue;
        };
        let kind = match (drawn.kind, node.id) {
            (NodeKind::Call, NodeId::Block(block)) => HotspotKind::Call {
                callee: flow.blocks[block].callee_name().span(),
            },
            _ => HotspotKind::Node,
        };
        push(kind, Scene::bounds(node), span);
    }
    let boundaries = scene.topology.cycle_boundaries.iter();
    for (boundary, region) in boundaries.zip(&scene.cycle_regions) {
        let cycle = Node {
            id: NodeId::Block(boundary.header),
            kind: NodeKind::Cycle,
        };
        if region.drawn
            && let Some(span) = root.node_span(part, &cycle)
        {
            push(HotspotKind::Node, region.bounds(), span);
        }
    }
    for label in &scene.labels {
        let LabelKind::Wire(list) = label.kind else {
            continue;
        };
        let bindings = scene.captions.bindings(list);
        for (position, rect) in name_rects(label, scene.captions.names(list)) {
            if let Some(binding) = bindings[position] {
                push(HotspotKind::Wire, rect, binding);
            }
        }
    }
}
