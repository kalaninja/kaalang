use kaalang::kaalang;

struct Counter(u32);

impl Counter {
    fn next(&mut self) -> Option<u32> {
        self.0 = 0;
        None
    }
}

impl Iterator for Counter {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.checked_sub(1)?;
        Some(self.0)
    }
}

#[kaalang]
fn for_iterator_trait(remaining: u32) -> u32 {
    #[action("Start with no visits.")]
    let mut visits = || 0;

    #[cycle("Visit every iterator item.")]
    |remaining| {
        for _ in Counter(remaining) {
            #[action("Count the visit.")]
            |&mut visits| *visits += 1;
        }
    };

    |visits| return visits;
}

#[test]
fn iteration_uses_the_trait_even_when_an_inherent_next_exists() {
    assert_eq!(Counter(3).next(), None);
    assert_eq!(for_iterator_trait(0), 0);
    assert_eq!(for_iterator_trait(3), 3);
}
