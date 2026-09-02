use kaalang::kaalang;

#[kaalang]
fn capture_alternative(condition: bool) -> u32 {
    #[question("Choose a result.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes result.")]
    |yes| -> result { 11 };

    #[action("Build the no result.")]
    |no| -> result { 29 };

    #[end]
    |result| {};
}

#[test]
fn alternative_producers_feed_the_same_end_input() {
    assert_eq!(capture_alternative(true), 11);
    assert_eq!(capture_alternative(false), 29);
}
