use kaalang::kaa;

#[kaa]
fn reversed_empty_cycle(mut checks: usize) -> usize {
    #[kaa::cycle("Repeat the check until its first branch leaves.")]
    let leave_1 = loop {
        #[kaa::question("Check once more?")]
        #[kaa::no("NO")]
        #[kaa::yes("YES")]
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
    assert_eq!(reversed_empty_cycle(0), 4);
    assert_eq!(reversed_empty_cycle(4), 5);
}
