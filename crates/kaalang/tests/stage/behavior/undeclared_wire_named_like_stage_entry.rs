use kaalang::kaa;
#[kaa]
fn undeclared_wire_named_like_stage_entry(go: ()) -> u8 {
    #[kaa::stage("Make a local wire.")]
    let next = |go| {
        #[kaa::action("Name a local wire after another entry.")]
        let finish = |go| 4u8;
        #[kaa::action("Use the local wire.")]
        |finish| {
            let _ = finish;
        };
        #[kaa::action("Select the next stage.")]
        let next = || {};
    };
    #[kaa::stage("Select the final stage.")]
    let finish = |next| {
        #[kaa::action("Produce the declared signal.")]
        let finish = |next| 8u8;
    };
    #[kaa::stage("Return.")]
    |finish| {
        |finish| return finish;
    };
}
#[test]
fn an_undeclared_wire_with_an_entry_name_stays_local() {
    assert_eq!(undeclared_wire_named_like_stage_entry(()), 8);
}
