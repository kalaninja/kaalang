use kaalang::kaa;

#[kaa]
fn borrowed_case_input(text: String) -> usize {
    #[kaa::choice("Borrow the text through the choice.")]
    #[kaa::case("Keep a long text whole.")]
    #[kaa::case("Keep the first character of a short text.")]
    let (long, short) = |&text| match text.len() {
        length if length > 3 => text.as_str(),
        _ => &text[..text.len().min(1)],
    };

    #[kaa::action("Measure the long text.")]
    let end = |long| long.len();

    #[kaa::action("Measure the short text.")]
    let end = |short| short.len();

    |end| return end;
}

#[test]
fn a_case_value_may_borrow_a_borrowed_input() {
    assert_eq!(borrowed_case_input("kaalang".into()), 7);
    assert_eq!(borrowed_case_input("ab".into()), 1);
    assert_eq!(borrowed_case_input(String::new()), 0);
}
