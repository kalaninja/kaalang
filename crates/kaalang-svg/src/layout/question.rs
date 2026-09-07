//! Anchors the first question branch below the node and the second at its right tip.

use super::{Node, Point};

pub(super) fn exit_anchor(node: &Node, branch: usize) -> Point {
    if branch == 0 {
        Point {
            x: node.x,
            y: node.y + node.height / 2,
        }
    } else {
        Point {
            x: node.x + node.width / 2,
            y: node.y,
        }
    }
}
