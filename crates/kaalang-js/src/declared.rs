//! What [`crate::declared_at`] answers: what a source text declares at an
//! offset.

use kaalang_compiler::{Declared, SourceFile};
use serde_json::{Value, json};

use crate::utf16;

/// The answer as JSON, or why the text cannot be read.
pub(crate) fn at(text: &str, cfg: &str, offset: u32) -> Result<String, String> {
    let file = SourceFile::parse(text, &cfg.parse()?).map_err(|error| error.to_string())?;
    let declared = match utf16::byte(text, offset).and_then(|byte| file.declared_at(byte)) {
        Some(Declared::Flow(position)) => json!({ "flow": position }),
        Some(Declared::Type(path)) => json!({ "type": path }),
        None => Value::Null,
    };
    Ok(declared.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{SOURCE, unit_of};

    #[test]
    fn a_declaration_reaches_javascript_as_json() {
        assert_eq!(
            at(SOURCE, "", unit_of("math::add")),
            Ok(r#"{"flow":0}"#.into())
        );
        assert_eq!(
            at(SOURCE, "", unit_of("Plain")),
            Ok(r#"{"type":"Plain"}"#.into())
        );
        assert_eq!(at(SOURCE, "", unit_of("🦀")), Ok("null".into()));
    }

    #[test]
    fn an_offset_past_the_end_declares_nothing() {
        let source = "#[kaa]\nfn settle(first: u8) -> u8 {\n    |first| return first;\n}";
        let end = u32::try_from(source.len()).unwrap();
        assert_eq!(at(source, "", end), Ok(r#"{"flow":0}"#.into()));
        assert_eq!(at(source, "", u32::MAX), Ok("null".into()));
    }
}
