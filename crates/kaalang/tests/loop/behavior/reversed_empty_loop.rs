use kaalang::kaalang;

#[kaalang]
fn reversed_empty_loop(mut checks: usize) -> usize {
    #[cycle("Repeat the check until its first branch leaves.")]
    let leave_1 = {
        #[question("Check once more?")]
        #[no("NO")]
        #[yes("YES")]
        let (leave_1, iterate_1) = |&mut checks| {
            *checks += 1;
            *checks < 4
        };

        |iterate_1| continue;
    };

    |leave_1, checks| return checks;
}

#[test]
fn an_empty_body_can_follow_the_second_answer() {
    assert_eq!(reversed_empty_loop(0), 4);
    assert_eq!(reversed_empty_loop(4), 5);
}
