use kaalang::kaa;

#[kaa]
fn borrowed_result(text: &str) -> &str {
    #[kaa::cycle("Expose a borrowed outer wire.")]
    let result = |text| loop {
        #[kaa::action("Borrow the text.")]
        let result = |text| text;
    };

    |result| return result;
}

#[test]
fn a_reference_to_a_captured_input_can_leave_the_cycle() {
    assert_eq!(borrowed_result("borrowed"), "borrowed");
}
