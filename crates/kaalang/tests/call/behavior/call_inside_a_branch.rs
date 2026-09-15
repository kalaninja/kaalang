use kaalang::kaalang;

fn double(value: i32) -> i32 {
    value * 2
}

fn negate(value: i32) -> i32 {
    -value
}

#[kaalang]
fn call_inside_a_branch(value: i32) -> i32 {
    #[question("Is the value positive?")]
    #[yes("YES")]
    #[no("NO")]
    let (positive, negative) = |&value| *value > 0;

    #[call("Double a positive value so it stays on the same side of zero.")]
    let scaled = |positive, value| double(value);

    #[call]
    let scaled = |negative, value| negate(value);

    |scaled| return scaled;
}

#[test]
fn a_call_joins_a_branch_by_capturing_its_control_wire() {
    assert_eq!(call_inside_a_branch(21), 42);
    assert_eq!(call_inside_a_branch(-7), 7);
}
