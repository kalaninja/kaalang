use kaalang::kaa;

/// The waiting case sits between the merged cases and its endless cycle is
/// written after their merge, but it never reaches that merge, so it cannot
/// separate the cases.
#[kaa]
fn case_waits_after_the_merge(value: u8) -> u8 {
    #[kaa::choice("Select a branch.")]
    #[kaa::case("Take the first branch.")]
    #[kaa::case("Wait forever.")]
    #[kaa::case("Take the second branch.")]
    let (first, wait, second) = |value| match value {
        0 => (),
        1 => (),
        _ => (),
    };

    #[kaa::action("Build the first value.")]
    let selected = |first| 1;

    #[kaa::action("Build the second value.")]
    let selected = |second| 3;

    #[kaa::action("Use the selected value.")]
    let result = |selected| selected + 1;

    #[kaa::cycle("Wait forever.")]
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
