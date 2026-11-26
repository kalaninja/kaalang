//! The source text the bindings' tests read.

use crate::utf16;

/// A flow and a type behind text that takes more bytes than UTF-16 units: a
/// Cyrillic letter takes two bytes and one unit, an emoji four bytes and two
/// units.
pub(crate) const SOURCE: &str = "// Пример 🦀\n#[kaa]\nfn settle(first: u8) -> u8 {\n    #[kaa::call]\n    let end = |first| math::add(first);\n\n    |end| return end;\n}\nstruct Plain;\n";

/// The UTF-16 offset of the first `text` in [`SOURCE`].
pub(crate) fn unit_of(text: &str) -> u32 {
    let byte = SOURCE.find(text).expect("the source has the text");
    utf16::Offsets::new(SOURCE).at(byte)
}
