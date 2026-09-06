use kaalang::kaalang;

#[allow(clippy::unused_unit)]
#[kaalang]
fn explicit_unit() -> () {
    #[action("Finish with a unit result.")]
    || -> result {};
}

#[test]
fn a_unit_result_wire_matches_the_explicit_return_type() {
    explicit_unit();
}
