use kaalang::kaalang;

#[kaalang]
fn discard_unnamed_input(_: u8) {
    #[action("Finish without the unnamed flow input.")]
    let end = || {};
}

#[test]
fn the_ignored_parameter_is_explicit() {
    discard_unnamed_input(2);
}
