use kaalang::kaa;

/// Two body routes export `larger`, and a question after the cycle splits its
/// continuation. The route taken inside the body orders nothing outside it.
#[kaa]
fn output_routes_before_a_question(value: u8, flag: u8) -> u8 {
    #[kaa::cycle("Sort the value.")]
    let (tiny, larger) = loop {
        #[kaa::question("Is it below ten?")]
        let (low, high) = |value| value < 10;

        #[kaa::question("Is it below five?")]
        let (below, small) = |low, value| value < 5;

        #[kaa::action("Mark a tiny value.")]
        let tiny = |below| {};

        #[kaa::action("Mark a small value.")]
        let larger = |small| {};

        #[kaa::action("Mark a high value.")]
        let larger = |high| {};
    };

    #[kaa::question("Is the flag even?")]
    let (even, odd) = |larger, flag| flag.is_multiple_of(2);

    #[kaa::action("Score the tiny value.")]
    let score = |tiny| 10;

    #[kaa::action("Score the even flag.")]
    let score = |even| 20;

    #[kaa::action("Keep the score.")]
    let result = |score| score;

    #[kaa::action("Score the odd flag.")]
    let result = |odd| 30;

    |result| return result;
}

#[test]
fn the_question_after_the_cycle_splits_one_output() {
    assert_eq!(output_routes_before_a_question(3, 0), 10);
    assert_eq!(output_routes_before_a_question(7, 0), 20);
    assert_eq!(output_routes_before_a_question(12, 1), 30);
}
