use kaalang::kaa;

#[kaa]
fn keep_tuple_in_single_output(input: u32) -> (u32, u32) {
    #[kaa::action("Build one tuple-valued output.")]
    let output = |input| (input, input + 1);

    |output| return output;
}

#[test]
fn a_single_identifier_binds_the_complete_value() {
    assert_eq!(keep_tuple_in_single_output(1), (1, 2));
}
