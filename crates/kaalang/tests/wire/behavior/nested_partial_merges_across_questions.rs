use kaalang::kaalang;

#[kaalang]
fn nested_partial_merges_across_questions(first: bool, second: bool, third: bool) -> u32 {
    #[question("Take the shortcut?")]
    let (shortcut, onward) = |first| first;

    #[question("Keep refining?")]
    let (refine, stop) = |onward, second| second;

    #[question("Refine once more?")]
    let (again, settle) = |refine, third| third;

    #[action("Start the inner value from two.")]
    let inner = |shortcut| 2;

    #[action("Start the inner value from three.")]
    let inner = |again| 3;

    #[action("Start the middle value from four.")]
    let middle = |settle| 4;

    #[action("Wrap the inner value.")]
    let middle = |inner| inner * 10 + 2;

    #[action("Wrap the middle value.")]
    let end = |middle| middle * 10 + 1;

    #[action("Stop at five.")]
    let end = |stop| 5;

    |end| return end;
}

#[test]
fn each_route_leaves_at_the_join_that_covers_it() {
    assert_eq!(
        nested_partial_merges_across_questions(true, false, false),
        221
    );
    assert_eq!(
        nested_partial_merges_across_questions(false, true, true),
        321
    );
    assert_eq!(
        nested_partial_merges_across_questions(false, true, false),
        41
    );
    assert_eq!(
        nested_partial_merges_across_questions(false, false, true),
        5
    );
}
