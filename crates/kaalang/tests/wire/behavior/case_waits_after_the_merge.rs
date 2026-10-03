use kaalang::kaalang;

/// The waiting case sits between the merged cases and its endless cycle is
/// written after their merge, but it never reaches that merge, so it cannot
/// separate the cases.
#[kaalang]
fn case_waits_after_the_merge(value: u8) -> u8 {
    #[choice("Select a branch.")]
    #[case("Take the first branch.")]
    #[case("Wait forever.")]
    #[case("Take the second branch.")]
    let (first, wait, second) = |value| match value {
        0 => (),
        1 => (),
        _ => (),
    };

    #[action("Build the first value.")]
    let selected = |first| 1;

    #[action("Build the second value.")]
    let selected = |second| 3;

    #[action("Use the selected value.")]
    let result = |selected| selected + 1;

    #[cycle("Wait forever.")]
    |wait| loop {
        continue;
    };

    |result| return result;
}

#[test]
fn the_merged_cases_return_while_the_waiting_case_stays_apart() {
    assert_eq!(case_waits_after_the_merge(0), 2);
    assert_eq!(case_waits_after_the_merge(2), 4);
}
