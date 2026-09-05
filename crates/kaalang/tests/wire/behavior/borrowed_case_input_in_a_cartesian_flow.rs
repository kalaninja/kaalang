use kaalang::kaalang;

#[kaalang]
fn borrowed_case_input_in_a_cartesian_flow(text: String, left: bool) -> usize {
    #[choice("Borrow the text through the choice.")]
    #[case("Keep a long text whole.")]
    #[case("Keep the first character of a short text.")]
    |&text| -> (long, short) {
        match text.len() {
            length if length > 3 => text.as_str(),
            _ => &text[..text.len().min(1)],
        }
    };

    #[question("Take the left path?")]
    |left| -> (a, b) { left };

    #[action("Measure the long text on the left path.")]
    |long, a| -> result { long.len() };

    #[action("Measure the long text on the right path.")]
    |long, b| -> result { long.len() + 10 };

    #[action("Measure the short text on the left path.")]
    |short, a| -> result { short.len() };

    #[action("Measure the short text on the right path.")]
    |short, b| -> result { short.len() + 10 };

    #[end]
    |result| {};
}

#[test]
fn a_guarded_case_value_may_borrow_a_borrowed_input() {
    assert_eq!(
        borrowed_case_input_in_a_cartesian_flow("kaalang".into(), true),
        7
    );
    assert_eq!(
        borrowed_case_input_in_a_cartesian_flow("kaalang".into(), false),
        17
    );
    assert_eq!(
        borrowed_case_input_in_a_cartesian_flow("ab".into(), true),
        1
    );
    assert_eq!(
        borrowed_case_input_in_a_cartesian_flow(String::new(), false),
        10
    );
}
