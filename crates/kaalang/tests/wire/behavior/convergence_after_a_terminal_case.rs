use kaalang::kaalang;

#[kaalang]
fn convergence_after_a_terminal_case(value: i8) -> &'static str {
    #[choice("Choose a path.")]
    #[case("Finish without a shared step.")]
    #[case("Build the left value.")]
    #[case("Build the right value.")]
    |value| -> (done, left, right) {
        match value {
            ..0 => (),
            0 => (),
            _ => (),
        }
    };

    #[action("Produce the direct result.")]
    |done| -> result { "done" };

    #[action("Build the left value.")]
    |left| -> selected { "left" };

    #[action("Build the right value.")]
    |right| -> selected { "right" };

    #[action("Use the selected value.")]
    |selected| -> result { selected };
}

#[test]
fn two_choice_branches_converge_after_a_terminal_case() {
    assert_eq!(convergence_after_a_terminal_case(-1), "done");
    assert_eq!(convergence_after_a_terminal_case(0), "left");
    assert_eq!(convergence_after_a_terminal_case(1), "right");
}
