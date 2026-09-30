use kaalang::kaalang;

#[kaalang]
fn local_parameter_assignment(value: u32) -> u32 {
    #[action("Reassign a mutable capture.")]
    let result = |mut value| {
        value += 8;
        value
    };

    |result| return result;
}

#[test]
fn a_mutable_capture_can_be_reassigned() {
    assert_eq!(local_parameter_assignment(1), 9);
}
