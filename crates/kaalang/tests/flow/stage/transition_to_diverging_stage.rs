use kaalang::kaa;

#[kaa]
fn transition_to_diverging_stage(go: ()) -> ! {
    #[kaa::stage("Enter the repeating stage.")]
    let repeat = |go| {
        #[kaa::action("Select the repeating stage.")]
        let repeat = |go| {};
    };

    #[kaa::stage("Repeat forever.")]
    |repeat| {
        #[kaa::cycle("Stay in this stage.")]
        |repeat| loop {
            continue;
        };
    };
}

#[test]
fn a_transition_can_enter_a_stage_that_diverges_in_a_nested_cycle() {
    let _: fn(()) -> ! = transition_to_diverging_stage;
}
