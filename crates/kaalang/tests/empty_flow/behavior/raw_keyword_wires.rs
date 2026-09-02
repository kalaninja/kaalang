use kaalang::kaalang;

#[kaalang]
fn raw_keyword_wires(r#type: u8) -> u8 {
    #[action("Rename the value.")]
    |r#type| -> r#match { r#type };

    #[action("Preserve the renamed value.")]
    |r#match| -> result { r#match };

    #[end]
    |result| {};
}

#[test]
fn the_raw_spelling_names_the_same_wire() {
    assert_eq!(raw_keyword_wires(7), 7);
}
