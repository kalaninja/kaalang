use kaalang::kaalang;

#[kaalang]
fn merged_break(stop: bool, remaining: usize) -> usize {
    #[cycle("Count down until either stopping condition is met.")]
    let result = |stop, mut remaining| {
        #[question("Stop immediately?")]
        let (leave, check) = |stop| stop;

        #[question("Has the countdown finished?")]
        let (leave, again) = |check, remaining| remaining == 0;

        |leave, remaining| break remaining;

        #[action("Advance the countdown.")]
        |again, &mut remaining| *remaining -= 1;
    };

    |result| return result;
}

#[test]
fn merged_question_outputs_share_one_break() {
    assert_eq!(merged_break(true, 3), 3);
    assert_eq!(merged_break(false, 0), 0);
    assert_eq!(merged_break(false, 3), 0);
}
