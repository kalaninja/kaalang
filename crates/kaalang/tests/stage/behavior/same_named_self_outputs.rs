use kaalang::kaalang;

#[kaalang]
fn same_named_self_outputs(go: u8) -> u8 {
    #[stage("Choose how to repeat.")]
    let (finish, go) = |go| {
        #[action("Read the entry.")]
        let current = |go| go;

        #[choice("Choose a route.")]
        #[case("Finish.")]
        #[case("Repeat from the left.")]
        #[case("Repeat from the right.")]
        let (done, left, right) = |current| match current {
            0 => (),
            1 => (),
            _ => (),
        };

        #[action("Keep the completed value.")]
        let finish = |done, current| current;

        #[action("Produce the left self-transition.")]
        let go = |left| 0;

        #[action("Produce the right self-transition.")]
        let go = |right| 0;
    };

    #[stage("Return the value.")]
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
