use kaalang::kaa;

#[kaa]
fn ordinary_wire_names(input: usize) -> usize {
    #[kaa::action("Produce an end wire.")]
    let end = |input| input + 1;

    #[kaa::cycle("Use completion-like names inside a cycle.")]
    let out = |end| loop {
        #[kaa::action("Produce a local result wire.")]
        let result = |end| end + 1;

        #[kaa::action("Produce a local out wire.")]
        let out = |result| result + 1;
    };

    #[kaa::action("Produce a root result wire.")]
    let result = |out| out + 1;

    |result| return result;
}

#[test]
fn end_out_and_result_are_ordinary_wires_at_each_scope() {
    assert_eq!(ordinary_wire_names(1), 5);
}
