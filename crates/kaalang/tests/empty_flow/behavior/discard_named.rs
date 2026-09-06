use kaalang::kaalang;

#[kaalang]
fn discard_named(_value: u8) {
    #[action("Finish without the named flow input.")]
    || -> result {};
}

#[test]
fn the_ignored_parameter_is_explicit() {
    discard_named(1);
}
