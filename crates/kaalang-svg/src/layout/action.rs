//! Sizes an action node.

use super::{NODE_LABEL_WIDTH, NODE_MIN_HEIGHT, block_dimensions};
use crate::text::RichText;

pub(super) fn dimensions(label: &RichText) -> (i32, i32, Vec<RichText>) {
    block_dimensions(label, NODE_LABEL_WIDTH, NODE_MIN_HEIGHT)
}
