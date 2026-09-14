use kaalang::kaalang;

#[kaalang]
fn nested_side_returns(flag: bool) {
    #[cycle("Repeat level 0.")]
    |flag| {
        #[question("Leave level 0?")]
        let (leave_0, stay_0) = |flag| flag;
        |leave_0| break;

        #[cycle("Repeat level 1.")]
        |stay_0, flag| {
            #[question("Leave level 1?")]
            let (stay_1, leave_1) = |flag| flag;
            |leave_1| break;

            #[cycle("Repeat level 2.")]
            |stay_1, flag| {
                #[question("Leave level 2?")]
                let (_stay_2, leave_2) = |flag| flag;
                |leave_2| break;
            };
        };
    };
    return;
}

#[test]
fn the_outer_cycle_can_exit_before_entering_the_nested_cycles() {
    nested_side_returns(true);
}
