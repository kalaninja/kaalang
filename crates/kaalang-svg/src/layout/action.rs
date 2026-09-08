//! Sizes an action node.

use super::{NODE_LABEL_WIDTH, NODE_WIDTH, block_dimensions};

pub(super) fn dimensions(label: &str) -> (i32, i32, Vec<String>) {
    block_dimensions(label, NODE_WIDTH, NODE_LABEL_WIDTH, 64)
}
