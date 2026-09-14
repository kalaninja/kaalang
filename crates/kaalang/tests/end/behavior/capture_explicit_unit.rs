use kaalang::kaalang;

#[kaalang]
fn capture_explicit_unit(input: ()) {
    |input| return input;
}

#[test]
fn return_can_capture_an_explicit_unit_wire() {
    capture_explicit_unit(());
}
