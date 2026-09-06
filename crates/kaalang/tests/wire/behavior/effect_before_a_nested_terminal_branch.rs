use kaalang::kaalang;

#[kaalang]
fn effect_before_a_nested_terminal_branch(
    outer: bool,
    inner: bool,
    log: &mut Vec<&'static str>,
) -> u32 {
    #[question("Take the nested path?")]
    |outer, &inner| -> (nested, direct) { outer };

    #[action("Record an effect independent of both questions.")]
    |log| -> logged { log.push("effect") };

    #[question("Finish early?")]
    |nested, inner| -> (early, late) { inner };

    #[action("Produce the early result.")]
    |early, logged| -> result { 1 };

    #[action("Build the late value.")]
    |late| -> selected { 2 };

    #[action("Build the direct value.")]
    |direct| -> selected { 3 };

    #[action("Use the selected value.")]
    |selected, logged| -> result { selected * 10 };
}

#[test]
fn the_effect_completes_before_every_branch_returns() {
    for (outer, inner, expected) in [
        (true, true, 1),
        (true, false, 20),
        (false, true, 30),
        (false, false, 30),
    ] {
        let mut log = Vec::new();
        assert_eq!(
            effect_before_a_nested_terminal_branch(outer, inner, &mut log),
            expected
        );
        assert_eq!(log, ["effect"]);
    }
}
