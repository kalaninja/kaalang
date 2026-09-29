use kaalang::kaalang;

#[kaalang]
fn raw_entry(r#go: u8) -> u8 {
    #[stage("Forward the entry.")]
    let finish = |go| {
        #[action("Forward the value.")]
        let finish = |go| go;
    };

    #[stage("Return the value.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn raw_and_plain_signal_spellings_resolve_to_one_stage() {
    assert_eq!(raw_entry(9), 9);
}
