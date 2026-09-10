use kaalang::kaalang;

#[kaalang]
fn reversed_empty_loop(mut checks: usize) -> usize {
    #[question("Check once more?")]
    #[no]
    #[yes]
    while (|&mut checks| {
        *checks += 1;
        *checks < 4
    }) {}

    #[action("Return the number of checks.")]
    let end = |checks| checks;
}

#[test]
fn an_empty_body_can_follow_the_second_answer() {
    assert_eq!(reversed_empty_loop(0), 4);
    assert_eq!(reversed_empty_loop(4), 5);
}
