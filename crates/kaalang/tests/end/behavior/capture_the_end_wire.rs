use kaalang::kaalang;

#[kaalang]
fn capture_the_end_wire(input: u32) -> u32 {
    #[action("Produce the result.")]
    let end = |input| input;

    #[action("Capture the ordinary `end` wire.")]
    let doubled = |end| end * 2;

    |doubled| return doubled;
}

#[test]
fn end_can_be_captured_before_the_flow_returns() {
    assert_eq!(capture_the_end_wire(3), 6);
}
