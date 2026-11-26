//! Where the incoming cards of a type diagram stand and what their headers
//! say, before any card is measured.

use std::{collections::BTreeSet, num::NonZeroUsize};

/// The row and column of each incoming card, in order, `groups` giving each
/// card's group: a card of another group than the card before it starts a new
/// row, and a group wraps into further rows after `columns` cards.
#[must_use]
pub fn card_cells<K: PartialEq>(groups: &[K], columns: NonZeroUsize) -> Vec<(usize, usize)> {
    let columns = columns.get();
    let mut cells = Vec::with_capacity(groups.len());
    let mut first = 0;
    for group in groups.chunk_by(PartialEq::eq) {
        cells.extend((0..group.len()).map(|index| (first + index / columns, index % columns)));
        first += group.len().div_ceil(columns);
    }
    cells
}

/// An incoming card's type for [`card_headers`]: its path from its crate's
/// root, and the name of its crate when that is not the focus's.
#[derive(Clone, Copy, Debug)]
pub struct CardPath<'a> {
    pub path: &'a str,
    pub other_crate: Option<&'a str>,
}

/// The header of each incoming card: a path from the focus's crate drops the
/// module segments it shares with `module`, the focus's module path with a
/// trailing `::`, or nothing at the crate root, and gets them back one by one
/// where it would read as another header or as `focus`, the focus's own; a
/// path from another crate keeps the crate's name and its whole path.
#[must_use]
pub fn card_headers(focus: &str, module: &str, incoming: &[CardPath<'_>]) -> Vec<String> {
    let mut headers: Vec<String> = incoming
        .iter()
        .map(|card| {
            if let Some(krate) = card.other_crate {
                return format!("{krate}::{}", card.path);
            }
            let shared: usize = module
                .split_inclusive("::")
                .zip(card.path.split_inclusive("::"))
                .take_while(|(left, right)| left == right)
                .map(|(segment, _)| segment.len())
                .sum();
            card.path[shared..].to_owned()
        })
        .collect();
    loop {
        let mut seen = BTreeSet::from([focus]);
        let collisions: BTreeSet<String> = headers
            .iter()
            .filter(|header| !seen.insert(header.as_str()))
            .cloned()
            .collect();
        let mut changed = false;
        for (card, header) in incoming.iter().zip(&mut headers) {
            if card.other_crate.is_some() || !collisions.contains(header.as_str()) {
                continue;
            }
            let prefix = &card.path[..card.path.len() - header.len()];
            let Some(prefix) = prefix.strip_suffix("::") else {
                continue;
            };
            let start = prefix.rfind("::").map_or(0, |at| at + 2);
            card.path[start..].clone_into(header);
            changed = true;
        }
        if !changed {
            return headers;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::{CardPath, card_cells, card_headers};

    #[test]
    fn each_group_starts_a_row_and_wraps_after_the_columns() {
        let two = NonZeroUsize::new(2).unwrap();
        assert_eq!(
            card_cells(&["lib", "lib", "lib", "tests"], two),
            [(0, 0), (0, 1), (1, 0), (2, 0)]
        );
    }

    #[test]
    fn an_incoming_header_drops_the_modules_it_shares_with_the_focus() {
        let card = |path, other_crate| CardPath { path, other_crate };
        assert_eq!(
            card_headers(
                "shop::orders::Order",
                "shop::orders::",
                &[
                    card("shop::orders::Line", None),
                    card("shop::payments::Payment", None),
                    card("shop::orders::Line", Some("external")),
                ],
            ),
            ["Line", "payments::Payment", "external::shop::orders::Line"]
        );
    }

    #[test]
    fn colliding_headers_get_their_modules_back() {
        let card = |path| CardPath {
            path,
            other_crate: None,
        };
        assert_eq!(
            card_headers(
                "shop::orders::Order",
                "shop::orders::",
                &[card("shop::orders::Note"), card("shop::Note")],
            ),
            ["orders::Note", "shop::Note"]
        );
    }
}
