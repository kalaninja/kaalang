use kaalang::kaalang;

#[kaalang]
fn reversed_empty_loop(mut checks: usize) -> usize {
    |&checks| loop {
        #[question("Check once more?")]
        #[no("NO")]
        #[yes("YES")]
        let (leave_1, _iterate_1) = |&mut checks| {
            *checks += 1;
            *checks < 4
        };

        |leave_1| break;
    };

    #[action("Return the number of checks.")]
    let end = |checks| checks;
}

#[test]
fn an_empty_body_can_follow_the_second_answer() {
    assert_eq!(reversed_empty_loop(0), 4);
    assert_eq!(reversed_empty_loop(4), 5);
}
