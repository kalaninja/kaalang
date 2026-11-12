use kaalang::kaa;

#[kaa]
fn same_named_self_outputs(go: u8) -> u8 {
    #[kaa::stage("Choose how to repeat.")]
    let (finish, go) = |go| {
        #[kaa::action("Read the entry.")]
        let current = |go| go;

        #[kaa::choice("Choose a route.")]
        #[kaa::case("Finish.")]
        #[kaa::case("Repeat from the left.")]
        #[kaa::case("Repeat from the right.")]
        let (done, left, right) = |current| match current {
            0 => (),
            1 => (),
            _ => (),
        };

        #[kaa::action("Keep the completed value.")]
        let finish = |done, current| current;

        #[kaa::action("Produce the left self-transition.")]
        let go = |left| 0;

        #[kaa::action("Produce the right self-transition.")]
        let go = |right| 0;
    };

    #[kaa::stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn alternative_self_outputs_merge_after_entry_shadowing() {
    for value in 0..=2 {
        assert_eq!(same_named_self_outputs(value), 0);
    }
}
