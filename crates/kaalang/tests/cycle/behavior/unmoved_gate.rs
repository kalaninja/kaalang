use kaalang::kaalang;

#[kaalang]
fn unmoved_gate(text: String) -> usize {
    #[cycle("Enter through an owned gate.")]
    let entered = |text| loop {
        #[action("Enter once.")]
        let entered = || {};
    };

    #[action("Use the gate after the cycle.")]
    let result = |entered, text| text.len();

    |result| return result;
}

#[test]
fn a_gate_leaves_its_owned_wire_in_place() {
    assert_eq!(unmoved_gate(String::from("four")), 4);
}
