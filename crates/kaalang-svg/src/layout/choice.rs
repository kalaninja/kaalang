//! Sizes a case node derived from a choice.

use super::{
    CASE_LABEL_WIDTH, CASE_TIP_HEIGHT, CASE_WIDTH, LABEL_FONT, LINE_HEIGHT, text::wrap_text,
};

pub(super) fn case_dimensions(label: &str) -> (i32, i32, Vec<String>) {
    let lines = wrap_text(label, CASE_LABEL_WIDTH, LABEL_FONT);
    let body_height = 52.max(28 + lines.len() as i32 * LINE_HEIGHT);
    (CASE_WIDTH, body_height + CASE_TIP_HEIGHT, lines)
}
