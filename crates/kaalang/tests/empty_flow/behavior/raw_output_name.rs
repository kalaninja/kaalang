use kaalang::kaalang;

#[kaalang]
fn raw_output_name(value: u8) -> u8 {
    #[action("Preserve the value.")]
    |value| -> r#result { value };
}

#[test]
fn the_raw_spelling_names_the_same_wire() {
    assert_eq!(raw_output_name(5), 5);
}
