use kaalang::kaa;

#[kaa]
fn middle_exit(limit: usize) -> Vec<usize> {
    #[kaa::action("Initialize the counter and log.")]
    let (mut initial_count, mut initial_log) = || (0, Vec::new());

    #[kaa::cycle("Record iterations through the requested limit.")]
    let done = loop {
        #[kaa::action("Record the start of the iteration.")]
        |&initial_count, &mut initial_log| initial_log.push(*initial_count);

        #[kaa::question("Stop before advancing?")]
        let (done, again) = |&initial_count, limit| *initial_count == limit;

        #[kaa::action("Advance and record the rest of the iteration.")]
        |again, &mut initial_count, &mut initial_log| {
            *initial_count += 1;
            initial_log.push(99);
        };

        |again| continue;
    };

    |done, initial_log| return initial_log;
}

#[test]
fn a_mid_body_exit_skips_the_remaining_work() {
    assert_eq!(middle_exit(0), [0]);
    assert_eq!(middle_exit(2), [0, 99, 1, 99, 2]);
}
