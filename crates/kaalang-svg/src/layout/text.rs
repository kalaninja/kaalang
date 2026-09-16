//! Estimates glyph widths and wraps labels to a pixel budget.

use unicode_segmentation::UnicodeSegmentation;

/// Approximate advance width of one character, in hundredths of an em, for the
/// sans-serif stack the stylesheet requests. The renderer has no font metrics,
/// so this estimate errs wide rather than letting a label leave its node.
fn advance(character: char) -> i32 {
    match character {
        '\t' => 200,
        ' ' | '.' | ',' | ':' | ';' | '!' | '|' | '\'' | '`' | 'i' | 'j' | 'l' | 'I' | '('
        | ')' | '[' | ']' | '{' | '}' | '/' | '\\' | '-' => 32,
        'm' | 'w' | 'M' | 'W' | '@' => 90,
        'A'..='Z' => 68,
        _ if character.is_ascii() => 56,
        // Latin-1, Greek, and Cyrillic behave like Latin; assume anything
        // beyond them, such as CJK, is full width.
        _ if (character as u32) < 0x0500 => 60,
        _ => 100,
    }
}

/// Estimates a grapheme cluster from its base character; joined marks add no width.
fn cluster_advance(cluster: &str) -> i32 {
    advance(
        cluster
            .chars()
            .next()
            .expect("a grapheme cluster has at least one character"),
    )
}

/// Estimated rendered width of `text` at `font_size`.
pub(super) fn text_width(text: &str, font_size: i32) -> i32 {
    text.graphemes(true).map(cluster_advance).sum::<i32>() * font_size / 100
}

/// The same, for clusters a caller has already split.
fn clusters_width(clusters: &[&str], font_size: i32) -> i32 {
    clusters.iter().copied().map(cluster_advance).sum::<i32>() * font_size / 100
}

/// Returns how many leading clusters fit within `budget`, at least one so that
/// wrapping always makes progress.
fn fitting_count(clusters: &[&str], budget: i32, font_size: i32) -> usize {
    let mut used = 0;
    for (count, cluster) in clusters.iter().enumerate() {
        used += cluster_advance(cluster);
        if used * font_size / 100 > budget {
            return count.max(1);
        }
    }

    clusters.len()
}

/// Wraps at the last fitting space, or between grapheme clusters for long words.
/// Preserves text except newline separators; an oversized cluster stays intact.
pub(super) fn wrap_text(text: &str, budget: i32, font_size: i32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let clusters = paragraph.graphemes(true).collect::<Vec<_>>();
        if clusters.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut start = 0;
        while clusters_width(&clusters[start..], font_size) > budget {
            let hard_end = start + fitting_count(&clusters[start..], budget, font_size);
            let preferred_break = clusters[start..hard_end]
                .iter()
                .rposition(|cluster| cluster.starts_with(char::is_whitespace))
                .map(|offset| start + offset + 1)
                .filter(|end| *end > start + 1);
            // A space past the budget is no break at all: taking it would draw
            // the line outside the node the budget measures.
            let end = preferred_break.unwrap_or(hard_end);
            lines.push(clusters[start..end].concat());
            start = end;
        }
        if start < clusters.len() {
            lines.push(clusters[start..].concat());
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::{text_width, wrap_text};
    use crate::layout::{LABEL_FONT, NODE_LABEL_WIDTH};

    /// Long words split only at grapheme boundaries, even below one cluster's width.
    #[test]
    fn wrapping_splits_between_graphemes_and_never_inside_one() {
        let word = "драконоподобный";
        let lines = wrap_text(word, 20, 14);

        assert!(lines.len() > 1);
        assert_eq!(lines.concat(), word);
        for line in &lines {
            assert!(text_width(line, 14) <= 20, "line exceeds: {line}");
        }

        // Narrower than one cluster, so only the cluster boundary can break.
        assert_eq!(wrap_text("👩‍💻👩‍💻", 8, 14), ["👩‍💻", "👩‍💻"]);
        assert_eq!(
            wrap_text("e\u{301}e\u{301}", 8, 14),
            ["e\u{301}", "e\u{301}"]
        );
    }

    #[test]
    fn wrapping_preserves_authored_whitespace() {
        let text = "  exact  spacing  ";

        assert_eq!(wrap_text(text, 40, 14).concat(), text);
    }

    #[test]
    fn wrapped_lines_fit_the_label_budget() {
        let label = "REJECT THE WWWWIDE APPLICATION IMMEDIATELY AND WITHOUT DELAY";
        let lines = wrap_text(label, NODE_LABEL_WIDTH, LABEL_FONT);

        assert!(lines.len() > 1);
        for line in &lines {
            assert!(
                text_width(line, LABEL_FONT) <= NODE_LABEL_WIDTH,
                "line exceeds the budget: {line}"
            );
        }
        assert_eq!(lines.concat(), label);
    }

    /// The space after a long word sits past the budget, so breaking there
    /// would draw the word and the space outside the node.
    #[test]
    fn wrapping_splits_a_long_word_rather_than_reaching_the_space_after_it() {
        let text = format!("{} tail", "W".repeat(32));
        let lines = wrap_text(&text, NODE_LABEL_WIDTH, LABEL_FONT);

        assert!(lines.len() > 1);
        assert_eq!(lines.concat(), text);
        for line in &lines {
            assert!(
                text_width(line, LABEL_FONT) <= NODE_LABEL_WIDTH,
                "line exceeds the budget: {line}"
            );
        }
    }
}
