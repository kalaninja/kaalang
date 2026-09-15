use kaalang::kaalang;

fn extend(total: &mut usize, text: &str) {
    *total += text.len();
}

#[kaalang]
fn call_a_borrowed_input(mut total: usize, text: String) -> usize {
    #[call("Add the length of the text to the total.")]
    let extended = |&mut total, &text| extend(total, text);

    #[action("Report the total.")]
    let end = |extended, total| total;

    |end| return end;
}

#[test]
fn a_call_passes_borrowed_inputs_as_references() {
    assert_eq!(call_a_borrowed_input(3, "kaalang".into()), 10);
}
