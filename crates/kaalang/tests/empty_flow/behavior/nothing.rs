use kaalang::kaalang;

#[kaalang]
fn nothing() {
    return;
}

#[test]
fn a_bare_return_finishes_a_unit_flow() {
    nothing();
}
