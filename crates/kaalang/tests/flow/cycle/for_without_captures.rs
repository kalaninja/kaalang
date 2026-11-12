use kaalang::kaa;

#[kaa]
fn for_without_captures() -> String {
    #[kaa::action("Start with an empty text.")]
    let mut text = || String::new();

    #[kaa::cycle("Knock three times.")]
    for _ in 0..3 {
        #[kaa::action("Knock once.")]
        |&mut text| text.push_str("knock ");
    }

    |text| return text;
}

#[test]
fn a_for_cycle_without_captures_iterates_its_own_items() {
    assert_eq!(for_without_captures(), "knock knock knock ");
}
