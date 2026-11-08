use kaalang::kaa;

#[kaa]
fn convergence_after_a_terminal_case(value: i8) -> &'static str {
    #[kaa::choice("Choose a branch.")]
    #[kaa::case("Finish without a shared step.")]
    #[kaa::case("Build the left value.")]
    #[kaa::case("Build the right value.")]
    let (done, left, right) = |value| match value {
        ..0 => (),
        0 => (),
        _ => (),
    };

    #[kaa::action("Produce the direct result.")]
    let end = |done| "done";

    #[kaa::action("Build the left value.")]
    let selected = |left| "left";

    #[kaa::action("Build the right value.")]
    let selected = |right| "right";

    #[kaa::action("Use the selected value.")]
    let end = |selected| selected;

    |end| return end;
}

#[test]
fn two_choice_branches_converge_after_a_terminal_case() {
    assert_eq!(convergence_after_a_terminal_case(-1), "done");
    assert_eq!(convergence_after_a_terminal_case(0), "left");
    assert_eq!(convergence_after_a_terminal_case(1), "right");
}
