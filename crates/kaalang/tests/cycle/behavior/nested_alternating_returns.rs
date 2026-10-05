use kaalang::kaalang;

#[kaalang]
fn nested_alternating_returns(flag: bool) {
    #[cycle("Repeat level 0.")]
    let leave_0 = |flag| loop {
        #[question("Leave level 0?")]
        let (stay_0, leave_0) = |flag| flag;
        #[cycle("Repeat level 1.")]
        let leave_1 = |stay_0| loop {
            #[question("Leave level 1?")]
            let (leave_1, stay_1) = |flag| flag;
            #[cycle("Repeat level 2.")]
            let leave_2 = |stay_1| loop {
                #[question("Leave level 2?")]
                let (stay_2, leave_2) = |flag| flag;
                |stay_2| continue;
            };
            |leave_2| continue;
        };
        |leave_1| continue;
    };
    |leave_0| return;
}

#[test]
fn the_outer_cycle_can_exit_before_entering_the_nested_cycles() {
    nested_alternating_returns(false);
}
