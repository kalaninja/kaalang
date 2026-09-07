use kaalang::kaalang;

#[kaalang]
fn raw_flow_input_name(value: u8) -> u8 {
    #[action("Keep the raw spelling of the flow input.")]
    |r#value| -> result { r#value };
}

#[test]
fn the_raw_spelling_names_the_same_wire() {
    assert_eq!(raw_flow_input_name(5), 5);
}
