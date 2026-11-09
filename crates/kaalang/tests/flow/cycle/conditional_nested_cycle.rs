use kaalang::kaa;

#[kaa]
fn conditional_nested_cycle(flag: bool) -> usize {
    #[kaa::cycle("Choose between a nested cycle and a direct result.")]
    let selected = |flag| loop {
        #[kaa::question("Enter the cycle?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (iterate_1, leave_1) = |flag| flag;

        #[kaa::action("Produce two.")]
        let selected = |leave_1| 2;

        #[kaa::cycle("Produce one.")]
        let selected = |iterate_1| loop {
            #[kaa::action("Produce one.")]
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
