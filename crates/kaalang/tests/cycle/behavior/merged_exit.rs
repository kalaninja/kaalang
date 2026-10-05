use kaalang::kaalang;

#[kaalang]
fn merged_exit(stop: bool, mut remaining: usize) -> usize {
    #[cycle("Count down until either stopping condition is met.")]
    let leave = loop {
        #[question("Stop immediately?")]
        let (leave, check) = |stop| stop;

        #[question("Has the countdown finished?")]
        let (leave, again) = |check, remaining| remaining == 0;

        #[action("Advance the countdown.")]
        |again, &mut remaining| *remaining -= 1;

        |again| continue;
    };

    |leave, remaining| return remaining;
}

#[test]
fn merged_question_outputs_share_one_exit() {
    assert_eq!(merged_exit(true, 3), 3);
    assert_eq!(merged_exit(false, 0), 0);
    assert_eq!(merged_exit(false, 3), 0);
}
