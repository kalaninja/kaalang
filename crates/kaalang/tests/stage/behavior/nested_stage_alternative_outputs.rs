use kaalang::kaalang;

#[kaalang]
fn nested_stage_alternative_outputs(go: u8) -> u8 {
    #[stage("Choose a nested result.")]
    let (finish, go) = |go| {
        #[action("Read the current entry.")]
        let current = |go| go;

        #[cycle("Select an outer result.")]
        let (finish, go) = |current| {
            #[cycle("Select an inner result.")]
            let (inner_finish, inner_again) = |current| {
                #[question("Is the value zero?")]
                let (inner_finish, inner_again) = |current| current == 0;
            };

            #[action("Forward the finished value.")]
            let finish = |inner_finish| 0u8;

            #[action("Count down before revisiting.")]
            let go = |inner_again, current| current - 1;
        };
    };

    #[stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn nested_alternative_outputs_select_the_stage_self_transition() {
    assert_eq!(nested_stage_alternative_outputs(0), 0);
    assert_eq!(nested_stage_alternative_outputs(3), 0);
}
