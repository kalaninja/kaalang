use kaalang::kaalang;

#[kaalang]
fn borrowed_case_input(text: String) -> usize {
    #[choice("Borrow the text through the choice.")]
    #[case("Keep a long text whole.")]
    #[case("Keep the first character of a short text.")]
    let (long, short) = |&text| match text.len() {
        length if length > 3 => text.as_str(),
        _ => &text[..text.len().min(1)],
    };

    #[action("Measure the long text.")]
    let result = |long| long.len();

    #[action("Measure the short text.")]
    let result = |short| short.len();
}

#[test]
fn a_case_value_may_borrow_a_borrowed_input() {
    assert_eq!(borrowed_case_input("kaalang".into()), 7);
    assert_eq!(borrowed_case_input("ab".into()), 1);
    assert_eq!(borrowed_case_input(String::new()), 0);
}
