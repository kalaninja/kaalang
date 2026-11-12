use kaalang::kaa;

struct Gate(u8);

#[kaa]
const fn const_noncopy_gate(gate: Gate) -> u8 {
    #[kaa::cycle("Read an owned gate without moving it.")]
    let result = |gate| loop {
        #[kaa::action("Borrow the original gate.")]
        let result = |&gate| gate.0;
    };

    |result| return result;
}

#[test]
fn a_const_cycle_gate_needs_no_copy_implementation() {
    const VALUE: u8 = const_noncopy_gate(Gate(7));
    assert_eq!(VALUE, 7);
}
