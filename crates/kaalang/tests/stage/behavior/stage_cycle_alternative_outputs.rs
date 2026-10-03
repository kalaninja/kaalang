use kaalang::kaalang;

#[kaalang]
fn stage_cycle_alternative_outputs(go: u8) -> u8 {
    #[stage("Count down.")]
    let (finish, go) = |go| {
        #[action("Read the current value.")]
        let current = |go| go;

        #[cycle("Choose whether to repeat the stage.")]
        let (finish, go) = |current| loop {
            #[choice("What happens to the value?")]
            #[case("Repeat this cycle.")]
            #[case("Finish the flow.")]
            #[case("Visit this stage again.")]
            let (retry, done, again) = |current| match current {
                255 => (),
                0 => (),
                _ => (),
            };

            |retry| continue;

            #[action("Keep the finished value.")]
            let finish = |done, current| current;

            #[action("Decrease the value.")]
            let go = |again, current| current - 1;
        };
    };

    #[stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn alternative_cycle_outputs_select_the_next_stage() {
    assert_eq!(stage_cycle_alternative_outputs(0), 0);
    assert_eq!(stage_cycle_alternative_outputs(3), 0);
}
