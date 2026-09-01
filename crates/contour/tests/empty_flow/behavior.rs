use contour::contour;

#[contour]
fn nothing() {
    #[end]
    || {};
}

#[allow(clippy::unused_unit)]
#[contour]
fn explicit_unit() -> () {
    #[end]
    || {};
}

#[contour]
fn discard_named(_value: u8) {
    #[end]
    || {};
}

#[contour]
fn discard_at_the_boundary(_: u8) {
    #[end]
    || {};
}

#[contour]
fn identity<T>(value: T) -> T {
    #[end]
    |value| {};
}

#[contour]
fn pair<T>(a: T, b: T) -> (T, T) {
    #[end]
    |a, b| {};
}

#[contour]
fn reverse_pair<T>(a: T, b: T) -> (T, T) {
    #[end]
    |b, a| {};
}

#[contour]
fn named_after_wildcard<T>(_: (), value: T) -> T {
    #[end]
    |value| {};
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
