use std::cell::Cell;

use kaalang::kaalang;

struct __FlowJoin1(u8);

#[kaalang]
fn nested_branch_passes_a_question_join(
    outer: bool,
    middle: bool,
    inner: bool,
    local_runs: &Cell<u32>,
    outer_runs: &Cell<u32>,
) -> u32 {
    #[question("Take the nested questions?")]
    |outer| -> (left, right) { outer };

    #[question("Refine the local value?")]
    |left, middle| -> (refine, direct) { middle };

    #[question("Take the local continuation?")]
    |refine, inner| -> (join, skip) { inner };

    #[action("Build the refined local value.")]
    |join| -> shared {
        let value: __FlowJoin1 = __FlowJoin1(1u8);
        value.0
    };

    #[action("Build the direct local value.")]
    |direct| -> shared { 2u8 };

    #[action("Run the local continuation and finish.")]
    |shared, local_runs| -> result {
        local_runs.set(local_runs.get() + 1);
        u32::from(shared) + 10
    };

    #[action("Skip the local continuation.")]
    |skip| -> ready { 3u8 };

    #[action("Build the outer value directly.")]
    |right| -> ready { 4u8 };

    #[action("Run the outer continuation and finish.")]
    |ready, outer_runs| -> result {
        outer_runs.set(outer_runs.get() + 1);
        u32::from(ready) + 100
    };
}

#[test]
fn a_nested_branch_passes_its_question_join_without_running_it() {
    for (outer, middle, inner, expected, runs) in [
        (true, true, true, 11, (1, 0)),
        (true, false, true, 12, (1, 0)),
        (true, true, false, 103, (0, 1)),
        (false, false, false, 104, (0, 1)),
    ] {
        let local_runs = Cell::new(0);
        let outer_runs = Cell::new(0);
        assert_eq!(
            nested_branch_passes_a_question_join(outer, middle, inner, &local_runs, &outer_runs),
            expected
        );
        assert_eq!((local_runs.get(), outer_runs.get()), runs);
    }
}
