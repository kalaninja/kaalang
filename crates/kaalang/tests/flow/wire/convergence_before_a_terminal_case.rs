use kaalang::kaa;

#[kaa]
fn convergence_before_a_terminal_case(case: u8) -> u32 {
    #[kaa::choice("Choose whether to continue.")]
    #[kaa::case("First continuing branch.")]
    #[kaa::case("Second continuing branch.")]
    #[kaa::case("Terminal case.")]
    let (first, second, done) = |case| match case {
        0 => (),
        1 => (),
        _ => (),
    };

    #[kaa::action("Build the first value.")]
    let selected = |first| 1;

    #[kaa::action("Build the second value.")]
    let selected = |second| 2;

    #[kaa::action("Produce the direct result.")]
    let end = |done| 99;

    #[kaa::action("Use a value from a continuing branch.")]
    let end = |selected| selected * 10;

    |end| return end;
}

#[test]
fn branches_converge_before_a_terminal_case() {
    assert_eq!(convergence_before_a_terminal_case(0), 10);
    assert_eq!(convergence_before_a_terminal_case(1), 20);
    assert_eq!(convergence_before_a_terminal_case(2), 99);
}
