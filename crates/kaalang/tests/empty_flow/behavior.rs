use kaalang::kaalang;

#[kaalang]
fn nothing() {
    #[end]
    || {};
}

#[allow(clippy::unused_unit)]
#[kaalang]
fn explicit_unit() -> () {
    #[end]
    || {};
}

#[kaalang]
fn discard_named(_value: u8) {
    #[end]
    || {};
}

#[kaalang]
fn discard_at_the_boundary(_: u8) {
    #[end]
    || {};
}

#[kaalang]
fn identity<T>(value: T) -> T {
    #[end]
    |value| {};
}

#[kaalang]
fn pair<T>(a: T, b: T) -> (T, T) {
    #[end]
    |a, b| {};
}

#[kaalang]
fn reverse_pair<T>(a: T, b: T) -> (T, T) {
    #[end]
    |b, a| {};
}

#[kaalang]
fn named_after_wildcard<T>(_: (), value: T) -> T {
    #[end]
    |value| {};
}

#[kaalang]
fn raw_source_name(value: u8) -> u8 {
    #[end]
    |r#value| {};
}

#[kaalang]
fn raw_output_name(value: u8) -> u8 {
    #[action("Preserve the value.")]
    |value| -> r#result { value };

    #[end]
    |result| {};
}

#[kaalang]
fn raw_ignored_output(value: u8) -> u8 {
    #[action("Preserve the value and ignore the marker.")]
    |value| -> (result, r#_ignored) { (value, ()) };

    #[end]
    |result| {};
}

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
fn zero_input_end_returns_unit_without_a_wire() {
    nothing();
    explicit_unit();
}

#[test]
fn ignored_parameters_are_explicit() {
    discard_named(1);
    discard_at_the_boundary(2);
}

#[test]
fn end_captures_sources_by_name_and_in_authored_order() {
    assert_eq!(identity(String::from("value")), "value");
    assert_eq!(pair(1, 2), (1, 2));
    assert_eq!(reverse_pair(1, 2), (2, 1));
    assert_eq!(named_after_wildcard((), 3), 3);
}

#[test]
fn raw_and_ordinary_identifier_spellings_name_the_same_wire() {
    assert_eq!(raw_source_name(4), 4);
    assert_eq!(raw_output_name(5), 5);
    assert_eq!(raw_ignored_output(6), 6);
    assert_eq!(raw_keyword_wires(7), 7);
}
