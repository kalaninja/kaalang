use kaalang::kaalang;
#[kaalang]
fn undeclared_wire_named_like_stage_entry(go: ()) -> u8 {
    #[stage("Make a local wire.")]
    let next = |go| {
        #[action("Name a local wire after another entry.")]
        let finish = |go| 4u8;
        #[action("Use the local wire.")]
        |finish| {
            let _ = finish;
        };
        #[action("Select the next stage.")]
        let next = || {};
    };
    #[stage("Select the final stage.")]
    let finish = |next| {
        #[action("Produce the declared signal.")]
        let finish = |next| 8u8;
    };
    #[stage("Return.")]
    |finish| {
        |finish| return finish;
    };
}
#[test]
fn an_undeclared_wire_with_an_entry_name_stays_local() {
    assert_eq!(undeclared_wire_named_like_stage_entry(()), 8);
}
