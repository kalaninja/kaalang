use kaalang::kaalang;

#[kaalang]
fn nothing() {
    #[end]
    || {};
}

#[test]
fn zero_input_end_returns_unit_without_a_wire() {
    nothing();
}
