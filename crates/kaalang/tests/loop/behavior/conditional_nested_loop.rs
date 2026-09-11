use kaalang::kaalang;

#[kaalang]
fn conditional_nested_loop(flag: bool) -> usize {
    loop {
        #[question("Enter the loop?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_1, leave_1) = |flag| flag;

        |leave_1| break;

        |iterate_1| loop {
            #[action("Return one.")]
            let end = || 1;
        };
    }

    #[action("Return two.")]
    let end = || 2;
}

#[test]
fn an_inner_loop_returns_through_the_terminal_merge() {
    assert_eq!(conditional_nested_loop(true), 1);
    assert_eq!(conditional_nested_loop(false), 2);
}
