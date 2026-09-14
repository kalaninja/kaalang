use kaalang::kaalang;

#[kaalang]
fn borrowed_result(text: &str) -> &str {
    #[cycle("Expose a borrowed cycle input.")]
    let result = |text| {
        |text| break text;
    };

    |result| return result;
}

#[test]
fn a_reference_to_a_captured_input_can_leave_the_cycle() {
    assert_eq!(borrowed_result("borrowed"), "borrowed");
}
