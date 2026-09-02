use kaalang::kaalang;

#[kaalang]
fn keep_tuple_in_single_output(input: u32) -> (u32, u32) {
    #[action("Build one tuple-valued output.")]
    |input| -> (output,) { (input, input + 1) };

    #[action("Produce the tuple.")]
    |output| -> result { output };

    #[end]
    |result| {};
}

#[test]
fn singleton_declaration_binds_the_complete_value() {
    assert_eq!(keep_tuple_in_single_output(1), (1, 2));
}
