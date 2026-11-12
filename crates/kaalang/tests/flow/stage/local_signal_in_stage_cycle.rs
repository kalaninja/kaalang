use kaalang::kaa;

#[kaa]
fn local_signal_in_stage_cycle(go: ()) -> u8 {
    #[kaa::stage("Keep a local signal in a cycle.")]
    let next = |go| {
        #[kaa::cycle("Produce a local value.")]
        let local = |go| loop {
            #[kaa::action("Use a name reserved for a later stage.")]
            let finish = |go| {};

            #[kaa::action("Complete the cycle without exporting that name.")]
            let local = |finish| {};
        };

        #[kaa::action("Select the next stage.")]
        let next = |local| {};
    };

    #[kaa::stage("Produce the final signal.")]
    let finish = |next| {
        #[kaa::action("Produce the declared signal.")]
        let finish = |next| 9u8;
    };

    #[kaa::stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn a_unit_wire_inside_a_cycle_does_not_select_a_stage() {
    assert_eq!(local_signal_in_stage_cycle(()), 9);
}
