use core::prelude::v1::test as kaa;

#[kaalang::kaa]
#[kaa]
fn nothing() {
    return;
}

#[test]
fn a_bare_return_finishes_a_unit_flow() {
    nothing();
}
