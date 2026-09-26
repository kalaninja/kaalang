use kaalang::kaalang;

#[kaalang]
fn merged_loop(select_first: bool) -> usize {
    #[question("Select the first cycle?")]
    let (first, second) = |select_first| select_first;

    #[cycle("Produce the first result.")]
    let result = |first| {
        #[action("Build the first value.")]
        let result = || 1;
    };

    #[cycle("Produce the second result.")]
    let result = |second| {
        #[action("Build the second value.")]
        let result = || 2;
    };

    |result| return result;
}

#[test]
fn mutually_exclusive_cycles_merge_their_outer_result_wire() {
    assert_eq!(merged_loop(true), 1);
    assert_eq!(merged_loop(false), 2);
}
