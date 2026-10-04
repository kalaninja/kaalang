//! Serializes an expanded cycle boundary.

use std::fmt::Write;

use super::{
    Node, TextAnchor, needs_composed_lines, write_composed_lines, write_lines, write_start_label,
    write_title,
};
use crate::layout::{
    CAP_CHAMFER, CYCLE_CAPTION_BASELINE, CYCLE_CAPTION_FONT, CYCLE_CAPTION_LINE_HEIGHT,
    CYCLE_CAPTION_PADDING_X, CycleRegion,
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
    write_start_label(svg, node);
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
    write_title(svg, &region.description);
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

/// A for-entry cuts its two upper corners at 45° and a for-end its two lower
/// ones. Both hold the cycle's description like an action's, so each for-end
/// names the for-entry it closes.
pub(super) fn write_cap(svg: &mut String, node: &Node, entry: bool) {
    let (half_width, half_height) = (node.width / 2, node.height / 2);
    let inner = half_width - CAP_CHAMFER;
    let (top, bottom, shoulder) = if entry {
        (inner, half_width, CAP_CHAMFER - half_height)
    } else {
        (half_width, inner, half_height - CAP_CHAMFER)
    };
    emit!(
        svg,
        "      <polygon class=\"node-shape\" points=\"-{top},-{half_height} {top},-{half_height} {half_width},{shoulder} {bottom},{half_height} -{bottom},{half_height} -{half_width},{shoulder}\"/>"
    );
    write_start_label(svg, node);
}
