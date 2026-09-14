use kaalang::kaalang;

#[kaalang]
fn result_is_an_ordinary_wire(input: u32) -> u32 {
    #[action("Increment the input.")]
    let result = |input| input + 1;

    #[action("Double the intermediate result.")]
    let end = |result| result * 2;

    |end| return end;
}

#[test]
fn result_can_be_captured_before_return_finishes_the_flow() {
    assert_eq!(result_is_an_ordinary_wire(3), 8);
}
