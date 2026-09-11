use kaalang::kaalang;

#[kaalang]
fn sequential_exits(before_limit: usize, after_limit: usize) -> Vec<&'static str> {
    #[action("Initialize the counter and log.")]
    let (mut count, mut log) = || (0, Vec::new());

    loop {
        #[question("Stop before the work?")]
        let (stop_before, work) = |&count, before_limit, &mut log| {
            log.push("check before");
            *count >= before_limit
        };

        |stop_before| break;

        #[action("Perform the work and advance the counter.")]
        let worked = |work, &mut count, &mut log| {
            log.push("work");
            *count += 1;
        };

        #[question("Stop after the work?")]
        let (stop_after, again) = |worked, &count, after_limit, &mut log| {
            log.push("check after");
            *count >= after_limit
        };

        |stop_after| break;

        #[action("Finish the iteration.")]
        |again, &mut log| log.push("repeat");
    }

    #[action("Continue after the loop.")]
    let end = |mut log| {
        log.push("done");
        log
    };
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
