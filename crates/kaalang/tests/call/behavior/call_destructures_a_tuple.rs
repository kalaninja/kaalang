use kaalang::kaalang;

fn divide(value: u32) -> (u32, u32) {
    (value / 3, value % 3)
}

#[kaalang]
fn call_destructures_a_tuple(value: u32) -> u32 {
    #[call("Divide the value by three.")]
    let (quotient, remainder) = |value| divide(value);

    #[action("Report both parts as one number.")]
    let end = |quotient, remainder| quotient * 10 + remainder;

    |end| return end;
}

#[test]
fn a_call_destructures_its_returned_tuple() {
    assert_eq!(call_destructures_a_tuple(7), 21);
}
