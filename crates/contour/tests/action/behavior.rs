use contour::contour;

#[contour]
fn run_action(input: u32) -> u32 {
    #[action("Increment the input.")]
    |input| -> output { input + 1 };
}

#[contour]
fn keep_tuple_in_single_output(input: u32) -> (u32, u32) {
    #[action("Build one tuple-valued output.")]
    |input| -> (output,) { (input, input + 1) };

    #[action("Return the tuple.")]
    |output| -> result { output };
}

#[contour]
fn split_nested_tuple(input: u32) -> ((u32, u32), u32) {
    #[action("Build two outputs, including a tuple.")]
    |input| -> (out1, out2) { ((input, input + 1), input + 2) };

    #[action("Return both outputs.")]
    |out1, out2| -> result { (out1, out2) };
}

#[test]
fn action_block_executes() {
    assert_eq!(run_action(1), 2);
}

#[test]
fn singleton_declaration_binds_the_complete_value() {
    assert_eq!(keep_tuple_in_single_output(1), (1, 2));
}

#[test]
fn multiple_outputs_destructure_only_the_outer_tuple() {
    assert_eq!(split_nested_tuple(1), ((1, 2), 3));
}
