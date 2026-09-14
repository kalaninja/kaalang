use kaalang::kaalang;

#[kaalang]
fn reversed_empty_loop(checks: usize) -> usize {
    #[cycle("Repeat the check until its first branch leaves.")]
    let checked = |mut checks| {
        #[question("Check once more?")]
        #[no("NO")]
        #[yes("YES")]
        let (leave_1, _iterate_1) = |&mut checks| {
            *checks += 1;
            *checks < 4
        };

        |leave_1, checks| break checks;
    };

    |checked| return checked;
}

#[test]
fn an_empty_body_can_follow_the_second_answer() {
    assert_eq!(reversed_empty_loop(0), 4);
    assert_eq!(reversed_empty_loop(4), 5);
}
