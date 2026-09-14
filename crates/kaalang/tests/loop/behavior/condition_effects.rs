use kaalang::kaalang;

#[kaalang]
fn condition_effects(checks: usize) -> usize {
    #[cycle("Repeat the check until it fails.")]
    let checked = |mut checks| {
        #[question("Check once more?")]
        #[yes("YES")]
        #[no("NO")]
        let (_iterate_1, leave_1) = |&mut checks| {
            *checks += 1;
            *checks < 4
        };

        |leave_1, checks| break checks;
    };

    |checked| return checked;
}

#[test]
fn an_empty_body_rechecks_and_includes_the_final_false_check() {
    assert_eq!(condition_effects(0), 4);
    assert_eq!(condition_effects(4), 5);
}
