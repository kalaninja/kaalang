use kaalang::kaalang;

#[kaalang]
fn effect_before_a_nested_terminal_branch(
    outer: bool,
    inner: bool,
    log: &mut Vec<&'static str>,
) -> u32 {
    #[action("Record the effect before branching.")]
    let logged = |log| log.push("effect");

    #[question("Take the nested branch?")]
    let (nested, direct) = |outer, &inner| outer;

    #[question("Finish early?")]
    let (early, late) = |nested, inner| inner;

    #[action("Produce the early result.")]
    let end = |early, logged| 1;

    #[action("Build the late value.")]
    let selected = |late| 2;

    #[action("Build the direct value.")]
    let selected = |direct| 3;

    #[action("Use the selected value.")]
    let end = |selected, logged| selected * 10;
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
