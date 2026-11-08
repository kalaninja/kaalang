use kaalang::kaa;

#[kaa]
fn early_result_then_cycle(first: bool, mut count: usize) -> usize {
    #[kaa::cycle("Select an early result or continue.")]
    let early_result = |first| loop {
        #[kaa::question("Check for an early result?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (produce_early, continue_counting) = |first| first;

        #[kaa::action("Continue to the counter.")]
        let early_result = |continue_counting| None;

        #[kaa::action("Produce the early result.")]
        let early_result = |produce_early| Some(99);
    };

    #[kaa::question("Was an early result selected?")]
    let (finish_early, count_more) = |&early_result| early_result.is_some();

    #[kaa::action("Extract the early result.")]
    let end = |finish_early, early_result| early_result.expect("the early route has a result");

    #[kaa::cycle("Count to three.")]
    let end = |count_more| loop {
        #[kaa::question("Has the count reached three?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (leave, iterate) = |&count| *count >= 3;

        #[kaa::action("Hand over the count.")]
        let end = |leave, count| count;

        #[kaa::action("Increment the count.")]
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
