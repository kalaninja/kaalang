use kaalang::kaalang;

#[kaalang]
fn split_nested_tuple(input: u32) -> ((u32, u32), u32) {
    #[action("Build two outputs, including a tuple.")]
    let (out1, out2) = |input| ((input, input + 1), input + 2);

    #[action("Produce the result pair.")]
    let end = |out1, out2| (out1, out2);
}

#[test]
fn multiple_outputs_destructure_only_the_outer_tuple() {
    assert_eq!(split_nested_tuple(1), ((1, 2), 3));
}
