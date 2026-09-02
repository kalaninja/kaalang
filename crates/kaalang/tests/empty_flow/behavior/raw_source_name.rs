use kaalang::kaalang;

#[kaalang]
fn raw_source_name(value: u8) -> u8 {
    #[end]
    |r#value| {};
}

#[test]
fn the_raw_spelling_names_the_same_wire() {
    assert_eq!(raw_source_name(4), 4);
}
