//! Serializes an expanded cycle boundary.

use std::fmt::Write;

use super::{
    Node, TextAnchor, escape, needs_composed_lines, write_composed_lines, write_label, write_lines,
};
use crate::layout::{
    CYCLE_CAPTION_BASELINE, CYCLE_CAPTION_FONT, CYCLE_CAPTION_LINE_HEIGHT, CYCLE_CAPTION_PADDING_X,
    CycleRegion, NODE_LABEL_PADDING_X,
};

const NODE_CORNER_RADIUS: i32 = 8;
const BOUNDARY_CORNER_RADIUS: i32 = 12;
const MARKER_INSET_X: i32 = 18;
const MARKER_BASELINE: i32 = 7;

pub(super) fn write_node(svg: &mut String, node: &Node) {
    let half_width = node.width / 2;
    let half_height = node.height / 2;
    emit!(
        svg,
        "      <rect class=\"node-shape\" x=\"-{half_width}\" y=\"-{half_height}\" width=\"{}\" height=\"{}\" rx=\"{NODE_CORNER_RADIUS}\"/>",
        node.width,
        node.height
    );
    write_label(
        svg,
        node,
        0,
        -half_width + NODE_LABEL_PADDING_X,
        TextAnchor::Start,
    );
    emit!(
        svg,
        "      <text class=\"loop-marker\" x=\"{}\" y=\"{MARKER_BASELINE}\" aria-hidden=\"true\">↻</text>",
        half_width - MARKER_INSET_X
    );
}

pub(super) fn write(svg: &mut String, region: &CycleRegion) {
    let width = region.right - region.left;
    let height = region.bottom - region.top;
    emit!(svg, "    <g>");
    emit!(
        svg,
        "      <title xml:space=\"preserve\">{}</title>",
        escape(&region.description)
    );
    emit!(
        svg,
        "    <rect class=\"cycle-boundary\" x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{height}\" rx=\"{BOUNDARY_CORNER_RADIUS}\"/>",
        region.left,
        region.top
    );
    if !region.caption.is_empty() {
        let x = region.right - CYCLE_CAPTION_PADDING_X;
        if needs_composed_lines(&region.caption) {
            let metrics = crate::text::block_metrics(
                &region.caption,
                CYCLE_CAPTION_FONT,
                CYCLE_CAPTION_LINE_HEIGHT,
            );
            write_composed_lines(
                svg,
                "    ",
                "cycle-caption",
                None,
                &region.caption,
                x,
                region.top + CYCLE_CAPTION_BASELINE.max(metrics.baselines[0]),
                CYCLE_CAPTION_FONT,
                CYCLE_CAPTION_LINE_HEIGHT,
                TextAnchor::End,
            );
            emit!(svg, "    </g>");
            return;
        }
        emit_inline!(
            svg,
            "    <text class=\"cycle-caption\" x=\"{x}\" y=\"{}\" text-anchor=\"end\" xml:space=\"preserve\">",
            region.top + CYCLE_CAPTION_BASELINE
        );
        write_lines(
            svg,
            &region.caption,
            x,
            CYCLE_CAPTION_FONT,
            CYCLE_CAPTION_LINE_HEIGHT,
        );
    }
    emit!(svg, "    </g>");
}
