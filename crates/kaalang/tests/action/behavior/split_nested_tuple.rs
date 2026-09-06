use kaalang::kaalang;

#[kaalang]
fn split_nested_tuple(input: u32) -> ((u32, u32), u32) {
    #[action("Build two outputs, including a tuple.")]
    |input| -> (out1, out2) { ((input, input + 1), input + 2) };

    #[action("Produce the result pair.")]
    |out1, out2| -> result { (out1, out2) };
}

#[test]
fn multiple_outputs_destructure_only_the_outer_tuple() {
    assert_eq!(split_nested_tuple(1), ((1, 2), 3));
}
