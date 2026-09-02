use kaalang::kaalang;

#[allow(clippy::unused_unit)]
#[kaalang]
fn explicit_unit() -> () {
    #[end]
    || {};
}

#[test]
fn zero_input_end_returns_unit_without_a_wire() {
    explicit_unit();
}
