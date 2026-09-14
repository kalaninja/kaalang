use kaalang::kaalang;

#[kaalang]
fn middle_exit(limit: usize) -> Vec<usize> {
    #[action("Initialize the counter and log.")]
    let (initial_count, initial_log) = || (0, Vec::new());

    #[cycle("Record iterations through the requested limit.")]
    let result_log = |limit, mut initial_count, mut initial_log| {
        #[action("Record the start of the iteration.")]
        |&initial_count, &mut initial_log| initial_log.push(*initial_count);

        #[question("Stop before advancing?")]
        let (done, again) = |&initial_count, limit| *initial_count == limit;

        |done, initial_log| break initial_log;

        #[action("Advance and record the rest of the iteration.")]
        |again, &mut initial_count, &mut initial_log| {
            *initial_count += 1;
            initial_log.push(99);
        };
    };

    |result_log| return result_log;
}

#[test]
fn a_mid_body_break_skips_the_remaining_work() {
    assert_eq!(middle_exit(0), [0]);
    assert_eq!(middle_exit(2), [0, 99, 1, 99, 2]);
}
