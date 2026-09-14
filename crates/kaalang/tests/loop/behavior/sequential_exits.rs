use kaalang::kaalang;

#[kaalang]
fn sequential_exits(before_limit: usize, after_limit: usize) -> Vec<&'static str> {
    #[action("Initialize the counter and log.")]
    let (initial_count, initial_log) = || (0, Vec::new());

    #[cycle("Run work between two stopping checks.")]
    let log = |before_limit, after_limit, mut initial_count, mut initial_log| {
        #[question("Stop before the work?")]
        let (stop_before, work) = |&initial_count, before_limit, &mut initial_log| {
            initial_log.push("check before");
            *initial_count >= before_limit
        };

        |stop_before, initial_log| break initial_log;

        #[action("Perform the work and advance the counter.")]
        let worked = |work, &mut initial_count, &mut initial_log| {
            initial_log.push("work");
            *initial_count += 1;
        };

        #[question("Stop after the work?")]
        let (stop_after, again) = |worked, &initial_count, after_limit, &mut initial_log| {
            initial_log.push("check after");
            *initial_count >= after_limit
        };

        |stop_after, initial_log| break initial_log;

        #[action("Finish the iteration.")]
        |again, &mut initial_log| initial_log.push("repeat");
    };

    #[action("Continue after the loop.")]
    let result = |mut log| {
        log.push("done");
        log
    };

    |result| return result;
}

#[test]
fn sequential_breaks_skip_later_work_and_share_one_continuation() {
    assert_eq!(sequential_exits(0, 5), ["check before", "done"]);
    assert_eq!(
        sequential_exits(5, 1),
        ["check before", "work", "check after", "done"]
    );
    assert_eq!(
        sequential_exits(1, 5),
        [
            "check before",
            "work",
            "check after",
            "repeat",
            "check before",
            "done",
        ]
    );
    assert_eq!(
        sequential_exits(5, 2),
        [
            "check before",
            "work",
            "check after",
            "repeat",
            "check before",
            "work",
            "check after",
            "done",
        ]
    );
}
