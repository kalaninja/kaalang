use kaalang::kaalang;

#[kaalang]
fn leading_end_path_convergence(value: i8) -> &'static str {
    #[choice("Choose a path.")]
    #[case("Reach End directly")]
    #[case("Build the left value")]
    #[case("Build the right value")]
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

    #[end]
    |result| {};
}

#[test]
fn two_choice_branches_converge_after_a_leading_end_path() {
    assert_eq!(leading_end_path_convergence(-1), "done");
    assert_eq!(leading_end_path_convergence(0), "left");
    assert_eq!(leading_end_path_convergence(1), "right");
}
