//! Sizes an action node.

use super::{LABEL_FONT, LINE_HEIGHT, NODE_LABEL_WIDTH, NODE_WIDTH, text::wrap_text};

pub(super) fn dimensions(label: &str) -> (i32, i32, Vec<String>) {
    let lines = wrap_text(label, NODE_LABEL_WIDTH, LABEL_FONT);
    let height = 64.max(30 + lines.len() as i32 * LINE_HEIGHT);
    (NODE_WIDTH, height, lines)
}
