use kaalang::kaalang;

#[kaalang]
fn keep_tuple_in_single_output(input: u32) -> (u32, u32) {
    #[action("Build one tuple-valued output.")]
    let output = |input| (input, input + 1);

    #[action("Produce the tuple.")]
    let end = |output| output;
}

#[test]
fn a_single_identifier_binds_the_complete_value() {
    assert_eq!(keep_tuple_in_single_output(1), (1, 2));
}
