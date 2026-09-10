use kaalang::kaalang;

#[kaalang]
fn collect_steps(enabled: bool, limit: usize) -> Vec<String> {
    #[question("Collect the steps?")]
    let (run, skip) = |enabled| enabled;

    #[action("Start an empty log.")]
    let mut log = |run| Vec::new();

    #[question("Are there more first steps?")]
    while (|&log, limit| log.len() < limit) {
        #[action("Build the first step.")]
        let step = || {
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
        |&mut log, step| log.push(step);
    }

    #[question("Are there more second steps?")]
    while (|&log, limit| log.len() < limit * 2) {
        #[question("Is the log length odd?")]
        let (odd, even) = |&log| log.len() % 2 == 1;
        #[action("Build an odd step.")]
        let step = |odd| String::from("b");
        #[action("Build an even step.")]
        let step = |even| String::from("c");
        #[action("Record the second step.")]
        |&mut log, step| log.push(step);
    }

    #[action("Return the log.")]
    let result = |log| log;

    #[action("Return an empty log.")]
    let result = |skip| Vec::new();
}

#[test]
fn each_iteration_and_sibling_loop_has_its_own_local_wires() {
    assert!(collect_steps(false, 3).is_empty());
    assert!(collect_steps(true, 0).is_empty());
    assert_eq!(collect_steps(true, 3), ["a", "a", "a", "b", "c", "b"]);
}
