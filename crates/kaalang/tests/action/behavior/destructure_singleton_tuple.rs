use kaalang::kaalang;

#[kaalang]
fn destructure_singleton_tuple(input: u32) -> u32 {
    #[action("Extract the tuple element.")]
    let (mut value,) = |input| (input,);

    #[action("Increment the extracted value.")]
    |&mut value| *value += 1;

    #[action("Return the changed value.")]
    let end = |value| value;
}

#[test]
fn a_singleton_pattern_extracts_one_mutable_wire() {
    assert_eq!(destructure_singleton_tuple(4), 5);
}
