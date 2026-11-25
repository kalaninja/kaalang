//! What [`crate::declared_ranges`] answers: where a source text declares what
//! kaalang draws.

use kaalang_compiler::Declared;
use serde_json::{Value, json};

use crate::utf16;

fn value(declared: Declared) -> Value {
    match declared {
        Declared::Flow(position, path) => json!({ "flow": position, "path": path }),
        Declared::Type(path) => json!({ "type": path }),
    }
}

/// The answer as JSON, or why the text cannot be read.
pub(crate) fn ranges(text: &str, cfg: &str) -> Result<String, String> {
    let file = crate::parse(text, cfg)?;
    let offsets = utf16::Offsets::new(text);
    let ranges: Vec<_> = file
        .declared_ranges()
        .into_iter()
        .map(|(range, declared)| {
            let (start, end) = offsets.range(&range);
            (start, end, value(declared))
        })
        .collect();
    serde_json::to_string(&ranges).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::unit_of;

    #[test]
    fn declarations_reach_javascript_as_utf16_ranges_in_json() {
        let plain = unit_of("struct Plain");
        let expected = json!([
            [unit_of("#[kaa]"), unit_of("\nstruct"), { "flow": 0, "path": "settle" }],
            [plain, plain + 13, { "type": "Plain" }],
        ]);
        assert_eq!(ranges(crate::fixture::SOURCE, ""), Ok(expected.to_string()));
    }
}
