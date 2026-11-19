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

/// The byte where the character holding each of the UTF-16 code units
/// `offsets` starts, or the end of `text` just past its last unit, in one pass
/// over it, in no particular order. An offset further on has none: JavaScript
/// passes a negative offset as one past any text.
pub(crate) fn bytes(text: &str, offsets: &[u32]) -> Vec<usize> {
    let mut wanted = offsets.to_vec();
    wanted.sort_unstable();
    let mut wanted = wanted.into_iter().peekable();
    let mut found = Vec::with_capacity(offsets.len());
    let mut units = 0;
    for (byte, character) in text.char_indices() {
        units += character.len_utf16() as u32;
        while wanted.next_if(|&offset| offset < units).is_some() {
            found.push(byte);
        }
    }
    found.extend(wanted.filter(|&offset| offset == units).map(|_| text.len()));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Cyrillic letter takes two bytes and one unit, an emoji four bytes and
    /// two units.
    const TEXT: &str = "// Пример 🦀\n";

    /// [`bytes`] for one offset, a character at a time.
    fn byte(text: &str, offset: u32) -> Option<usize> {
        let mut units = 0;
        for (byte, character) in text.char_indices() {
            units += character.len_utf16() as u32;
            if units > offset {
                return Some(byte);
            }
        }
        (units == offset).then_some(text.len())
    }

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
    fn many_offsets_find_the_bytes_one_finds() {
        let offsets: Vec<u32> = (0..16).rev().collect();
        let mut one_by_one: Vec<usize> = offsets.iter().filter_map(|&at| byte(TEXT, at)).collect();
        let mut together = bytes(TEXT, &offsets);
        one_by_one.sort_unstable();
        together.sort_unstable();
        assert_eq!(together, one_by_one);
    }

    #[test]
    fn a_cursor_stands_at_the_end_of_the_text_and_nowhere_past_it() {
        let end = Offsets::new(TEXT).at(TEXT.len());
        assert_eq!(byte(TEXT, end), Some(TEXT.len()));
        assert_eq!(byte(TEXT, end + 1), None);
    }
}
