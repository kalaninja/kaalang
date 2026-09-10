#![deny(unused_mut)]

use kaalang::kaalang;

#[kaalang]
fn local_mutability_of_outputs(input: String, number: u32) -> (String, u32, u32) {
    #[action("Produce immutable wires.")]
    let (text, count) = |input, number| (input, number);

    #[action("Change the owned text and a local copy of the count.")]
    let (mut changed, incremented) = |mut text, mut count| {
        text.push('!');
        count += 1;
        (text, count)
    };

    #[action("Return both counts and the changed text.")]
    let mut end = |changed, incremented, count| (changed, incremented, count);
}

#[test]
fn local_mutability_needs_no_mutable_producer_and_output_permission_may_go_unused() {
    assert_eq!(
        local_mutability_of_outputs("a".into(), 4),
        ("a!".into(), 5, 4)
    );
}
