use kaalang::kaalang;

#[kaalang]
fn discard_at_the_boundary(_: u8) {
    #[end]
    || {};
}

#[test]
fn the_ignored_parameter_is_explicit() {
    discard_at_the_boundary(2);
}
