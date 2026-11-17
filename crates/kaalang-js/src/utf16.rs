//! Offsets between a text's bytes, as Rust counts them, and its UTF-16 code
//! units, as JavaScript counts them.

/// The UTF-16 offset of every character boundary of a text, by its byte.
pub(crate) struct Offsets(Vec<u32>);

impl Offsets {
    pub(crate) fn new(text: &str) -> Self {
        let mut offsets = vec![0; text.len() + 1];
        let mut offset = 0;
        for (byte, character) in text.char_indices() {
            offsets[byte] = offset;
            offset += character.len_utf16() as u32;
        }
        offsets[text.len()] = offset;
        Self(offsets)
    }

    /// The UTF-16 offset of `byte`, a character boundary.
    pub(crate) fn at(&self, byte: usize) -> u32 {
        self.0[byte]
    }
}

/// The byte where the character holding UTF-16 code unit `offset` starts, the
/// end of `text` just past its last unit, or nothing further on: JavaScript
/// passes a negative offset as one past any text.
pub(crate) fn byte(text: &str, offset: u32) -> Option<usize> {
    let mut units = 0;
    for (byte, character) in text.char_indices() {
        units += character.len_utf16() as u32;
        if units > offset {
            return Some(byte);
        }
    }
    (units == offset).then_some(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Cyrillic letter takes two bytes and one unit, an emoji four bytes and
    /// two units.
    const TEXT: &str = "// Пример 🦀\n";

    #[test]
    fn a_wide_character_counts_its_utf16_units() {
        let offsets = Offsets::new(TEXT);
        assert_eq!(offsets.at(TEXT.find('🦀').unwrap()), 10);
        assert_eq!(offsets.at(TEXT.find('\n').unwrap()), 12);
    }

    #[test]
    fn a_cursor_on_either_unit_of_a_pair_stands_at_its_character() {
        let emoji = TEXT.find('🦀').unwrap();
        assert_eq!(byte(TEXT, 10), Some(emoji));
        assert_eq!(byte(TEXT, 11), Some(emoji));
        assert_eq!(byte(TEXT, 12), Some(emoji + '🦀'.len_utf8()));
    }

    #[test]
    fn a_cursor_stands_at_the_end_of_the_text_and_nowhere_past_it() {
        let end = Offsets::new(TEXT).at(TEXT.len());
        assert_eq!(byte(TEXT, end), Some(TEXT.len()));
        assert_eq!(byte(TEXT, end + 1), None);
    }
}
