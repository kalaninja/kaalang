use kaalang::kaalang;

#[kaalang]
fn order_the_flow_output(input: u32) -> (u32, u32, u32) {
    #[action("Build three values.")]
    |input| -> (first, second, third) { (input, input + 1, input + 2) };

    #[action("Order the three values.")]
    |third, first, second| -> result { (third, first, second) };
}

#[test]
fn one_block_orders_the_flow_output() {
    assert_eq!(order_the_flow_output(1), (3, 1, 2));
}
