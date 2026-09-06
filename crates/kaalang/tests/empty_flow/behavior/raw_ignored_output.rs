use kaalang::kaalang;

#[kaalang]
fn raw_ignored_output(value: u8) -> u8 {
    #[action("Preserve the value and ignore the marker.")]
    |value| -> (result, r#_ignored) { (value, ()) };
}

#[test]
fn the_raw_spelling_names_the_same_wire() {
    assert_eq!(raw_ignored_output(6), 6);
}
