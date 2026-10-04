use kaalang::kaalang;

#[allow(non_upper_case_globals)]
const item: u32 = 2;

#[kaalang]
fn for_item_constant() -> u32 {
    #[action("Start with no visits.")]
    let mut visits = || 0;

    #[cycle("Visit three items.")]
    for _ in 0..3_u32 {
        #[action("Count the visit.")]
        |&mut visits| *visits += 1;
    }

    |visits| return visits;
}

#[test]
fn a_constant_named_item_does_not_change_the_generated_pattern() {
    assert_eq!(item, 2);
    assert_eq!(for_item_constant(), 3);
}
