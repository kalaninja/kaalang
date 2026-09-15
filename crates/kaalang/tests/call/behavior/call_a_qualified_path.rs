use kaalang::kaalang;

#[kaalang]
fn call_a_qualified_path(text: &str) -> u32 {
    #[call]
    let parsed = |text| <u32 as core::str::FromStr>::from_str(text);

    #[action("Take the parsed number, or zero when the text is not one.")]
    let end = |parsed| parsed.unwrap_or_default();

    |end| return end;
}

#[test]
fn a_callee_may_name_a_qualified_self_type() {
    assert_eq!(call_a_qualified_path("7"), 7);
    assert_eq!(call_a_qualified_path("seven"), 0);
}
