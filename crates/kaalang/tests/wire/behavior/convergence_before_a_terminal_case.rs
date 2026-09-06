use kaalang::kaalang;

#[kaalang]
fn convergence_before_a_terminal_case(case: u8) -> u32 {
    #[choice("Choose whether to continue.")]
    #[case("First continuing branch.")]
    #[case("Second continuing branch.")]
    #[case("Terminal case.")]
    |case| -> (first, second, done) {
        match case {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the first value.")]
    |first| -> selected { 1 };

    #[action("Build the second value.")]
    |second| -> selected { 2 };

    #[action("Produce the direct result.")]
    |done| -> result { 99 };

    #[action("Use a value from a continuing branch.")]
    |selected| -> result { selected * 10 };

    #[end]
    |result| {};
}

#[test]
fn branches_converge_before_a_terminal_case() {
    assert_eq!(convergence_before_a_terminal_case(0), 10);
    assert_eq!(convergence_before_a_terminal_case(1), 20);
    assert_eq!(convergence_before_a_terminal_case(2), 99);
}
