use kaalang::kaalang;

#[kaalang]
fn condition_effects(mut checks: usize) -> usize {
    |&checks| loop {
        #[question("Check once more?")]
        #[yes("YES")]
        #[no("NO")]
        let (_iterate_1, leave_1) = |&mut checks| {
            *checks += 1;
            *checks < 4
        };

        |leave_1| break;
    };

    #[action("Return the number of checks.")]
    let end = |checks| checks;
}

#[test]
fn an_empty_body_rechecks_and_includes_the_final_false_check() {
    assert_eq!(condition_effects(0), 4);
    assert_eq!(condition_effects(4), 5);
}
