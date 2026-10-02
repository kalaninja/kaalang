use kaalang::kaalang;

#[kaalang]
fn collect_steps(enabled: bool, limit: usize) -> Vec<String> {
    #[question("Collect the steps?")]
    let (run, skip) = |enabled| enabled;

    #[action("Start an empty log.")]
    let mut initial_log = |run| Vec::new();

    #[cycle("Collect the first steps.")]
    let leave_1 = |initial_log| {
        #[question("Are there more first steps?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_1, leave_1) = |&initial_log, &limit| initial_log.len() < *limit;

        #[action("Build the first step.")]
        let step = |iterate_1| {
            let mut text = String::new();
            'native: for value in [0, 1, 2] {
                if value == 0 {
                    continue 'native;
                }
                text.push('a');
                break 'native;
            }
            text
        };

        #[action("Record the first step.")]
        |&mut initial_log, step| initial_log.push(step);

        |iterate_1| continue;
    };

    #[cycle("Collect the second steps.")]
    let end = |leave_1| {
        #[question("Are there more second steps?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_2, leave_2) = |&initial_log, &limit| initial_log.len() < *limit * 2;

        #[action("Hand over the collected log.")]
        let end = |leave_2, initial_log| initial_log;

        #[question("Is the log length odd?")]
        let (odd, even) = |iterate_2, &initial_log| initial_log.len() % 2 == 1;

        #[action("Build an odd step.")]
        let step = |odd| String::from("b");

        #[action("Build an even step.")]
        let step = |even| String::from("c");

        #[action("Record the second step.")]
        |&mut initial_log, step| initial_log.push(step);

        |iterate_2| continue;
    };

    #[action("Return an empty log.")]
    let end = |skip| Vec::new();

    |end| return end;
}

#[test]
fn each_iteration_and_sibling_cycle_has_its_own_local_wires() {
    assert!(collect_steps(false, 3).is_empty());
    assert!(collect_steps(true, 0).is_empty());
    assert_eq!(collect_steps(true, 3), ["a", "a", "a", "b", "c", "b"]);
}
