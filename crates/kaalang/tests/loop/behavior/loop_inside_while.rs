use kaalang::kaalang;

#[kaalang]
fn loop_inside_while(flag: bool) -> usize {
    #[question("Enter the loop?")]
    while (|flag| flag) {
        loop {
            #[action("Return one.")]
            let end = || 1;
        }
    }

    #[action("Return two.")]
    let end = || 2;
}

#[test]
fn an_inner_loop_returns_through_the_terminal_merge() {
    assert_eq!(loop_inside_while(true), 1);
    assert_eq!(loop_inside_while(false), 2);
}
