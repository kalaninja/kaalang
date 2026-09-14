//! Serializes an expanded cycle boundary.

use std::fmt::Write;

use super::{Node, escape, write_label, write_lines};
use crate::layout::{CYCLE_CAPTION_LINE_HEIGHT, LoopRegion};

pub(super) fn write_node(svg: &mut String, node: &Node) {
    let half_width = node.width / 2;
    let half_height = node.height / 2;
    emit!(
        svg,
        "      <rect class=\"node-shape\" x=\"-{half_width}\" y=\"-{half_height}\" width=\"{}\" height=\"{}\" rx=\"8\"/>",
        node.width,
        node.height
    );
    write_label(svg, node, 0, -half_width + 16);
    emit!(
        svg,
        "      <text class=\"loop-marker\" x=\"{}\" y=\"7\" aria-hidden=\"true\">↻</text>",
        half_width - 18
    );
}

pub(super) fn write(svg: &mut String, region: &LoopRegion) {
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
        "    <rect class=\"cycle-boundary\" x=\"{}\" y=\"{}\" width=\"{width}\" height=\"{height}\" rx=\"12\"/>",
        region.left,
        region.top
    );
    if !region.caption.is_empty() {
        let x = region.right - 12;
        emit_inline!(
            svg,
            "    <text class=\"cycle-caption\" x=\"{x}\" y=\"{}\" text-anchor=\"end\" xml:space=\"preserve\">",
            region.top + 14
        );
        write_lines(svg, &region.caption, x, CYCLE_CAPTION_LINE_HEIGHT);
    }
    emit!(svg, "    </g>");
}
