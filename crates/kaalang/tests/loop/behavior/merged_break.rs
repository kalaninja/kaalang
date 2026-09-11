use kaalang::kaalang;

#[kaalang]
fn merged_break(stop: bool, mut remaining: usize) -> usize {
    loop {
        #[question("Stop immediately?")]
        let (leave, check) = |stop| stop;

        #[question("Has the countdown finished?")]
        let (leave, again) = |check, remaining| remaining == 0;

        |leave| break;

        #[action("Advance the countdown.")]
        |again, &mut remaining| *remaining -= 1;
    }

    #[action("Return the remaining count.")]
    let end = |remaining| remaining;
}

#[test]
fn merged_question_outputs_share_one_break() {
    assert_eq!(merged_break(true, 3), 3);
    assert_eq!(merged_break(false, 0), 0);
    assert_eq!(merged_break(false, 3), 0);
}
