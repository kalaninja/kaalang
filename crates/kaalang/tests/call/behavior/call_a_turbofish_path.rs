use kaalang::kaalang;

#[kaalang]
fn call_a_turbofish_path(text: &str) -> u32 {
    #[call]
    let parsed = |text| str::parse::<u32>(text);

    #[action("Take the parsed number, or zero when the text is not one.")]
    let end = |parsed| parsed.unwrap_or_default();

    |end| return end;
}

#[test]
fn a_callee_may_carry_generic_arguments() {
    assert_eq!(call_a_turbofish_path("7"), 7);
    assert_eq!(call_a_turbofish_path("seven"), 0);
}
