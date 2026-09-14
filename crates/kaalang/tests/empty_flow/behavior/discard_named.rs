use kaalang::kaalang;

#[kaalang]
fn discard_named(_value: u8) {
    || return;
}

#[test]
fn a_zero_capture_return_can_ignore_a_named_parameter() {
    discard_named(1);
}
