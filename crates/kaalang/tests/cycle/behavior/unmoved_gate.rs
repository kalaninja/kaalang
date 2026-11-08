use kaalang::kaa;

#[kaa]
fn unmoved_gate(text: String) -> usize {
    #[kaa::cycle("Enter through an owned gate.")]
    let entered = |text| loop {
        #[kaa::action("Enter once.")]
        let entered = || {};
    };

    #[kaa::action("Use the gate after the cycle.")]
    let result = |entered, text| text.len();

    |result| return result;
}

#[test]
fn a_gate_leaves_its_owned_wire_in_place() {
    assert_eq!(unmoved_gate(String::from("four")), 4);
}
