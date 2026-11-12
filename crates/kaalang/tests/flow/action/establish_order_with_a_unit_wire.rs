use kaalang::kaa;

#[kaa]
fn establish_order_with_a_unit_wire(value: u32) -> u32 {
    #[kaa::action("Log flow entry.")]
    let entered = || println!("start");

    #[kaa::action("Continue once the flow has been entered.")]
    let end = |entered, value| value + 1;

    |end| return end;
}

#[test]
fn a_unit_valued_wire_orders_two_actions() {
    assert_eq!(establish_order_with_a_unit_wire(1), 2);
}
