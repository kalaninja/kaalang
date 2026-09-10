use kaalang::kaalang;

#[kaalang]
fn nothing() {
    #[action("Finish without doing anything.")]
    let end = || {};
}

#[test]
fn a_flow_without_wires_still_produces_its_end_wire() {
    nothing();
}
