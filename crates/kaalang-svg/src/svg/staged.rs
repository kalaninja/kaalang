//! Writes the measured preparation and stage parts as one accessible SVG.

use std::fmt::Write;

use kaalang_compiler::Analysis;
use kaalang_compiler::topology::{NodeId, NodeKind};

use crate::layout::{Point, StagedScene};

use super::{ARROW_MIDPOINT, ARROW_SIZE, STROKE_WIDTH, describe, escape, serialize_with_ids};

const RETURN_ARROW_SIZE: i32 = 8;

pub(crate) fn serialize_staged(scene: &StagedScene, root: &Analysis, flow_name: &str) -> String {
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" role=\"img\" aria-labelledby=\"kaalang-title\" aria-describedby=\"kaalang-description\"><title id=\"kaalang-title\">kaalang diagram for {}</title><desc id=\"kaalang-description\">{}</desc><rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n",
        scene.width,
        scene.height,
        scene.width,
        scene.height,
        escape(flow_name),
        escape(&staged_description(scene, root)),
    );
    emit_inline!(
        svg,
        "<defs><marker id=\"stage-return-arrow\" viewBox=\"0 0 {ARROW_SIZE} {ARROW_SIZE}\" refX=\"{ARROW_SIZE}\" refY=\"{ARROW_MIDPOINT}\" markerWidth=\"{RETURN_ARROW_SIZE}\" markerHeight=\"{RETURN_ARROW_SIZE}\" orient=\"auto\"><path d=\"M 0 0 L {ARROW_SIZE} {ARROW_MIDPOINT} L 0 {ARROW_SIZE} Z\" fill=\"#1f2937\"/></marker></defs>\n<g class=\"stage-connections\" fill=\"none\" stroke=\"#1f2937\" stroke-width=\"{STROKE_WIDTH}\">\n"
    );
    for points in &scene.connections {
        write_route(&mut svg, points, false);
    }
    if let Some(route) = &scene.return_route {
        write_route(&mut svg, route, true);
    }
    svg.push_str("</g>\n");
    for (part, (local, placement)) in scene.parts.iter().enumerate() {
        let (x, y) = (placement.x, placement.y);
        let nested = serialize_with_ids(local, flow_name, Some(part));
        let _ = writeln!(svg, "<g transform=\"translate({x} {y})\">{nested}</g>");
    }
    svg.push_str("</svg>\n");
    svg
}

fn write_route(svg: &mut String, points: &[Point], arrow: bool) {
    svg.push_str("<path d=\"");
    for (index, point) in points.iter().enumerate() {
        let command = if index == 0 { 'M' } else { 'L' };
        let _ = write!(svg, "{command} {} {} ", point.x, point.y);
    }
    svg.push('"');
    if arrow {
        svg.push_str(" marker-end=\"url(#stage-return-arrow)\"");
    }
    svg.push_str("/>\n");
}

fn staged_description(scene: &StagedScene, root: &Analysis) -> String {
    let mut description = scene.direct_entry.map_or_else(
        || "The leftmost flow selects the initial stage. Transitions return to the stage entry rail without repeating the initial flow. ".to_owned(),
        |entry| format!(
            "The flow starts directly at stage {}. Transitions return to the stage entry rail. ",
            root.stages[entry].entry_alias,
        ),
    );
    for (part, (local, _)) in scene.parts.iter().enumerate() {
        if part > 0 {
            let stage = &root.stages[part - 1];
            let _ = write!(description, "Stage {}. ", stage.entry_alias);
        }
        description.push_str(&describe(local));
        description.push(' ');
        let flow = if part == 0 {
            &root.flow
        } else {
            &root.stages[part - 1].analysis.flow
        };
        for node in &local.topology.nodes {
            if node.kind != NodeKind::Transition {
                continue;
            }
            let NodeId::Block(block) = node.id else {
                continue;
            };
            if let Some(destination) = flow.blocks[block].transition_target {
                let _ = write!(
                    description,
                    "Transition to stage {}. ",
                    root.stages[destination].entry_alias,
                );
            }
        }
    }
    description.trim_end().to_owned()
}
