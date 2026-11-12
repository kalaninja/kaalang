use kaalang::kaa;

#[kaa]
fn mutable_self_output(go: u8) -> u8 {
    #[kaa::stage("Increment until two.")]
    let (finish, go) = |go| {
        #[kaa::action("Read the entry.")]
        let current = |go| go;

        #[kaa::question("Reached two?")]
        let (done, again) = |current| current >= 2;

        #[kaa::action("Keep the result.")]
        let finish = |done, current| current;

        #[kaa::action("Create a mutable self-transition value.")]
        let mut go = |again, current| current;

        #[kaa::action("Increment the outgoing value.")]
        |&mut go| *go += 1;
    };

    #[kaa::stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn a_mutable_self_output_becomes_the_next_immutable_entry() {
    assert_eq!(mutable_self_output(0), 2);
    assert_eq!(mutable_self_output(3), 3);
}
