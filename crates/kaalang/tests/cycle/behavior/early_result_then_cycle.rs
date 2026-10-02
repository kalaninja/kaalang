use kaalang::kaalang;

#[kaalang]
fn early_result_then_cycle(first: bool, mut count: usize) -> usize {
    #[cycle("Select an early result or continue.")]
    let early_result = |first| {
        #[question("Check for an early result?")]
        #[yes("YES")]
        #[no("NO")]
        let (produce_early, continue_counting) = |first| first;

        #[action("Continue to the counter.")]
        let early_result = |continue_counting| None;

        #[action("Produce the early result.")]
        let early_result = |produce_early| Some(99);
    };

    #[question("Was an early result selected?")]
    let (finish_early, count_more) = |&early_result| early_result.is_some();

    #[action("Extract the early result.")]
    let end = |finish_early, early_result| early_result.expect("the early route has a result");

    #[cycle("Count to three.")]
    let end = |count_more| {
        #[question("Has the count reached three?")]
        #[yes("YES")]
        #[no("NO")]
        let (leave, iterate) = |&count| *count >= 3;

        #[action("Hand over the count.")]
        let end = |leave, count| count;

        #[action("Increment the count.")]
        |iterate, &mut count| *count += 1;

        |iterate| continue;
    };

    |end| return end;
}

#[test]
fn an_early_result_skips_the_later_cycle() {
    assert_eq!(early_result_then_cycle(true, 0), 99);
    assert_eq!(early_result_then_cycle(false, 0), 3);
    assert_eq!(early_result_then_cycle(true, 1), 99);
    assert_eq!(early_result_then_cycle(false, 1), 3);
    assert_eq!(early_result_then_cycle(true, 5), 99);
    assert_eq!(early_result_then_cycle(false, 5), 5);
}
