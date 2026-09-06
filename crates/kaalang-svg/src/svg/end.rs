//! Serializes the implicit end node.

use super::{Node, write_boundary_shape};

pub(super) fn name() -> String {
    "End".to_owned()
}

pub(super) fn write(svg: &mut String, node: &Node) {
    write_boundary_shape(svg, node);
    svg.push_str("      <text class=\"label\" y=\"5\">End</text>\n");
}
