use kaalang::kaalang;

#[kaalang]
fn condition_effects(mut checks: usize) -> usize {
    #[question("Check once more?")]
    while (|&mut checks| {
        *checks += 1;
        *checks < 4
    }) {}

    #[action("Return the number of checks.")]
    let end = |checks| checks;
}

#[test]
fn an_empty_body_rechecks_and_includes_the_final_false_check() {
    assert_eq!(condition_effects(0), 4);
    assert_eq!(condition_effects(4), 5);
}
