use kaalang::kaalang;

#[kaalang]
fn work_below_a_flow_input_end(end: u32, other: u32) -> u32 {
    #[action("Combine the ordinary `end` input with another value.")]
    let result = |end, other| end + other;

    |result| return result;
}

#[test]
fn an_input_named_end_does_not_finish_the_flow() {
    assert_eq!(work_below_a_flow_input_end(3, 4), 7);
}
