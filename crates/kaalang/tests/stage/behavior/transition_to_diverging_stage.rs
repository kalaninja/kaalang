use kaalang::kaalang;

#[kaalang]
fn transition_to_diverging_stage(go: ()) -> ! {
    #[stage("Enter the repeating stage.")]
    let repeat = |go| {
        #[action("Select the repeating stage.")]
        let repeat = |go| {};
    };

    #[stage("Repeat forever.")]
    |repeat| {
        #[cycle("Stay in this stage.")]
        |repeat| loop {
            continue;
        };
    };
}

#[test]
fn a_transition_can_enter_a_stage_that_diverges_in_a_nested_cycle() {
    let _: fn(()) -> ! = transition_to_diverging_stage;
}
