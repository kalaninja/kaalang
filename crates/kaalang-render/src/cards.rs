//! Where the incoming cards of a type diagram stand, before any card is
//! measured.

use std::num::NonZeroUsize;

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

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::card_cells;

    #[test]
    fn each_group_starts_a_row_and_wraps_after_the_columns() {
        let two = NonZeroUsize::new(2).unwrap();
        assert_eq!(
            card_cells(&["lib", "lib", "lib", "tests"], two),
            [(0, 0), (0, 1), (1, 0), (2, 0)]
        );
    }
}
