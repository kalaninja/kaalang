use std::cell::Cell;

use kaalang::kaalang;

/// Counts its drops, so the flow can see when each item goes away.
struct Item<'a>(&'a Cell<usize>);

impl Drop for Item<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[kaalang]
fn for_wildcard_drops_each_item(items: Vec<Item<'_>>, dropped: &Cell<usize>) -> Vec<usize> {
    #[action("Start with no observations.")]
    let mut seen = || Vec::new();

    #[cycle("Visit each item without binding it.")]
    |items| {
        for _ in items {
            #[action("Record how many items have been dropped so far.")]
            |dropped, &mut seen| {
                seen.push(dropped.get());
            };
        }
    };

    |seen| return seen;
}

#[test]
fn a_wildcard_item_drops_before_the_body_runs() {
    let dropped = Cell::new(0);
    let items = vec![Item(&dropped), Item(&dropped), Item(&dropped)];
    assert_eq!(for_wildcard_drops_each_item(items, &dropped), [1, 2, 3]);
}
