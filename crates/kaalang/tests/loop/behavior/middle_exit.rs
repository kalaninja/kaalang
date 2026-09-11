use kaalang::kaalang;

#[kaalang]
fn middle_exit(limit: usize) -> Vec<usize> {
    #[action("Initialize the counter and log.")]
    let (mut count, mut log) = || (0, Vec::new());

    loop {
        #[action("Record the start of the iteration.")]
        |&count, &mut log| log.push(*count);

        #[question("Stop before advancing?")]
        let (done, again) = |&count, limit| *count == limit;

        |done| break;

        #[action("Advance and record the rest of the iteration.")]
        |again, &mut count, &mut log| {
            *count += 1;
            log.push(99);
        };
    }

    #[action("Return the log after leaving the loop.")]
    let end = |log| log;
}

#[test]
fn a_mid_body_break_skips_the_remaining_work() {
    assert_eq!(middle_exit(0), [0]);
    assert_eq!(middle_exit(2), [0, 99, 1, 99, 2]);
}
