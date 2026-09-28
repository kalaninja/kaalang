use kaalang::kaalang;

#[kaalang]
fn mutable_self_output(go: u8) -> u8 {
    #[stage("Increment until two.")]
    let (finish, go) = |go| {
        #[action("Read the entry.")]
        let current = |go| go;

        #[question("Reached two?")]
        let (done, again) = |current| current >= 2;

        #[action("Keep the result.")]
        let finish = |done, current| current;

        #[action("Create a mutable self-transition value.")]
        let mut go = |again, current| current;

        #[action("Increment the outgoing value.")]
        |&mut go| *go += 1;
    };

    #[stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn a_mutable_self_output_becomes_the_next_immutable_entry() {
    assert_eq!(mutable_self_output(0), 2);
    assert_eq!(mutable_self_output(3), 3);
}
