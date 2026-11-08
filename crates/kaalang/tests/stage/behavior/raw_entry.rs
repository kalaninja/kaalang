use kaalang::kaa;

#[kaa]
fn raw_entry(r#go: u8) -> u8 {
    #[kaa::stage("Forward the entry.")]
    let finish = |go| {
        #[kaa::action("Forward the value.")]
        let finish = |go| go;
    };

    #[kaa::stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn raw_and_plain_signal_spellings_resolve_to_one_stage() {
    assert_eq!(raw_entry(9), 9);
}
