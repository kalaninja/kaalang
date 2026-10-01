//! Sizes a call node.

use super::{CALL_BAR_INSET, NODE_LABEL_PADDING_X, NODE_MIN_HEIGHT, NODE_WIDTH, block_dimensions};
use crate::text::RichText;

/// A call loses horizontal space on both sides to its bars.
const CALL_LABEL_WIDTH: i32 = NODE_WIDTH - 2 * (CALL_BAR_INSET + NODE_LABEL_PADDING_X);

pub(super) fn dimensions(label: &RichText) -> (i32, i32, Vec<RichText>) {
    block_dimensions(label, NODE_WIDTH, CALL_LABEL_WIDTH, NODE_MIN_HEIGHT)
}
