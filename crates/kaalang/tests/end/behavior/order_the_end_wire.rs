use kaalang::kaalang;

#[kaalang]
fn order_the_end_wire(input: u32) -> (u32, u32, u32) {
    #[action("Build three values.")]
    let (first, second, third) = |input| (input, input + 1, input + 2);

    #[action("Order the three values.")]
    let end = |third, first, second| (third, first, second);

    |end| return end;
}

#[test]
fn one_block_orders_the_end_wire() {
    assert_eq!(order_the_end_wire(1), (3, 1, 2));
}
