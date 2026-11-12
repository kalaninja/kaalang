use kaalang::kaa;

#[kaa]
fn stage_cycle_alternative_outputs(go: u8) -> u8 {
    #[kaa::stage("Count down.")]
    let (finish, go) = |go| {
        #[kaa::action("Read the current value.")]
        let current = |go| go;

        #[kaa::cycle("Choose whether to repeat the stage.")]
        let (finish, go) = |current| loop {
            #[kaa::choice("What happens to the value?")]
            #[kaa::case("Repeat this cycle.")]
            #[kaa::case("Finish the flow.")]
            #[kaa::case("Visit this stage again.")]
            let (retry, done, again) = |current| match current {
                255 => (),
                0 => (),
                _ => (),
            };

            |retry| continue;

            #[kaa::action("Keep the finished value.")]
            let finish = |done, current| current;

            #[kaa::action("Decrease the value.")]
            let go = |again, current| current - 1;
        };
    };

    #[kaa::stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn alternative_cycle_outputs_select_the_next_stage() {
    assert_eq!(stage_cycle_alternative_outputs(0), 0);
    assert_eq!(stage_cycle_alternative_outputs(3), 0);
}
