use kaalang::kaalang;

#[kaalang]
fn condition_effects(mut checks: usize) -> usize {
    #[cycle("Repeat the check until it fails.")]
    let leave_1 = {
        #[question("Check once more?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_1, leave_1) = |&mut checks| {
            *checks += 1;
            *checks < 4
        };

        |iterate_1| continue;
    };

    |leave_1, checks| return checks;
}

#[test]
fn an_empty_body_rechecks_and_includes_the_final_false_check() {
    assert_eq!(condition_effects(0), 4);
    assert_eq!(condition_effects(4), 5);
}
