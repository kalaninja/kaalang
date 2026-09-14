use kaalang::kaalang;

#[kaalang]
fn conditional_nested_loop(flag: bool) -> usize {
    #[cycle("Choose between a nested cycle and a direct result.")]
    let result = |flag| {
        #[question("Enter the loop?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_1, leave_1) = |flag| flag;

        #[action("Produce two.")]
        let selected = |leave_1| 2;

        #[cycle("Produce one.")]
        let selected = |iterate_1| {
            #[action("Produce one.")]
            let one = || 1;

            |one| break one;
        };

        |selected| break selected;
    };

    |result| return result;
}

#[test]
fn an_inner_cycle_result_reaches_the_terminal_merge() {
    assert_eq!(conditional_nested_loop(true), 1);
    assert_eq!(conditional_nested_loop(false), 2);
}
