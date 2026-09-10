use kaalang::kaalang;

#[kaalang]
fn establish_order_with_a_unit_wire(value: u32) -> u32 {
    #[action("Log flow entry.")]
    let entered = || println!("start");

    #[action("Continue once the flow has been entered.")]
    let end = |entered, value| value + 1;
}

#[test]
fn a_unit_valued_wire_orders_two_actions() {
    assert_eq!(establish_order_with_a_unit_wire(1), 2);
}
