use kaalang::kaalang;

#[kaalang]
fn convergence_before_a_terminal_case(case: u8) -> u32 {
    #[choice("Choose whether to continue.")]
    #[case("First continuing branch.")]
    #[case("Second continuing branch.")]
    #[case("Terminal case.")]
    let (first, second, done) = |case| match case {
        0 => (),
        1 => (),
        _ => (),
    };

    #[action("Build the first value.")]
    let selected = |first| 1;

    #[action("Build the second value.")]
    let selected = |second| 2;

    #[action("Produce the direct result.")]
    let end = |done| 99;

    #[action("Use a value from a continuing branch.")]
    let end = |selected| selected * 10;

    |end| return end;
}

#[test]
fn branches_converge_before_a_terminal_case() {
    assert_eq!(convergence_before_a_terminal_case(0), 10);
    assert_eq!(convergence_before_a_terminal_case(1), 20);
    assert_eq!(convergence_before_a_terminal_case(2), 99);
}
