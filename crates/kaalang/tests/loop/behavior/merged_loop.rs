use kaalang::kaalang;

#[kaalang]
fn merged_loop(first: bool, second: bool) -> usize {
    #[question("Enter immediately?")]
    let (enter, check) = |first| first;

    #[question("Enter after checking?")]
    let (enter, skip) = |check, second| second;

    #[action("Skip the loop.")]
    let end = |skip| 0;

    |enter| loop {
        #[action("Finish inside the loop.")]
        let end = || 1;
    };
}

#[test]
fn merged_question_outputs_share_one_loop_entry() {
    assert_eq!(merged_loop(false, false), 0);
    assert_eq!(merged_loop(false, true), 1);
    assert_eq!(merged_loop(true, false), 1);
    assert_eq!(merged_loop(true, true), 1);
}
