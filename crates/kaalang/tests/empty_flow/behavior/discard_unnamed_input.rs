use kaalang::kaalang;

#[kaalang]
fn discard_unnamed_input(_: u8) {
    return ();
}

#[test]
fn an_explicit_unit_return_can_ignore_an_unnamed_parameter() {
    discard_unnamed_input(2);
}
