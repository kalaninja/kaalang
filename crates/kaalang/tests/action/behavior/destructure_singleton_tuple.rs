use kaalang::kaa;

#[kaa]
fn destructure_singleton_tuple(input: u32) -> u32 {
    #[kaa::action("Extract the tuple element.")]
    let (mut value,) = |input| (input,);

    #[kaa::action("Increment the extracted value.")]
    |&mut value| *value += 1;

    |value| return value;
}

#[test]
fn a_singleton_pattern_extracts_one_mutable_wire() {
    assert_eq!(destructure_singleton_tuple(4), 5);
}
