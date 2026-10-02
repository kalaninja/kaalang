use kaalang::kaalang;

#[kaalang]
fn conditional_nested_cycle(flag: bool) -> usize {
    #[cycle("Choose between a nested cycle and a direct result.")]
    let selected = |flag| {
        #[question("Enter the cycle?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_1, leave_1) = |flag| flag;

        #[action("Produce two.")]
        let selected = |leave_1| 2;

        #[cycle("Produce one.")]
        let selected = |iterate_1| {
            #[action("Produce one.")]
            let selected = || 1;
        };
    };

    |selected| return selected;
}

#[test]
fn an_inner_cycle_result_reaches_the_terminal_merge() {
    assert_eq!(conditional_nested_cycle(true), 1);
    assert_eq!(conditional_nested_cycle(false), 2);
}
