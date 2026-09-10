use kaalang::kaalang;

#[kaalang]
fn capture_explicit_unit(input: ()) {
    #[action("Preserve the explicit unit wire.")]
    let end = |input| input;
}

#[test]
fn end_can_capture_an_explicit_unit_wire() {
    capture_explicit_unit(());
}
