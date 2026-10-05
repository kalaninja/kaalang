use kaalang::kaalang;

#[kaalang]
fn for_reversed_range(from: u32) -> Vec<u32> {
    #[action("Start with an empty countdown.")]
    let mut countdown = || Vec::new();

    #[cycle("Count down to one.")]
    |from| {
        for step in (1..=from).rev() {
            #[action("Record the step.")]
            |step, &mut countdown| countdown.push(step);
        }
    };

    |countdown| return countdown;
}

#[test]
fn a_reversed_range_yields_its_items_from_the_top() {
    assert_eq!(for_reversed_range(0), Vec::<u32>::new());
    assert_eq!(for_reversed_range(3), vec![3, 2, 1]);
}
