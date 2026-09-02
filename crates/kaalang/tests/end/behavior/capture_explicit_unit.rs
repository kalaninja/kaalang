use kaalang::kaalang;

#[kaalang]
fn capture_explicit_unit(input: ()) {
    #[action("Preserve the explicit unit wire.")]
    |input| -> result { input };

    #[end]
    |result| {};
}

#[test]
fn end_can_capture_an_explicit_unit_wire() {
    capture_explicit_unit(());
}
