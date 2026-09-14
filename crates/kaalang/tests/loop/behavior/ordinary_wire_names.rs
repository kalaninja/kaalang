use kaalang::kaalang;

#[kaalang]
fn ordinary_wire_names(input: usize) -> usize {
    #[action("Produce an end wire.")]
    let end = |input| input + 1;

    #[cycle("Use completion-like names inside a cycle.")]
    let out = |end| {
        #[action("Produce a local result wire.")]
        let result = |end| end + 1;

        #[action("Produce a local out wire.")]
        let out = |result| result + 1;

        |out| break out;
    };

    #[action("Produce a root result wire.")]
    let result = |out| out + 1;

    |result| return result;
}

#[test]
fn end_out_and_result_are_ordinary_wires_at_each_scope() {
    assert_eq!(ordinary_wire_names(1), 5);
}
