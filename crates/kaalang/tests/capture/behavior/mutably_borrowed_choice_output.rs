#![deny(unused_mut)]

use kaalang::kaalang;

/// A case value declared `mut` may be mutated through a mutable borrow.
#[kaalang]
fn mutably_borrowed_choice_output(input: Option<String>) -> usize {
    #[choice("Was text supplied?")]
    #[case("Text supplied.")]
    #[case("No text.")]
    let (mut text, absent) = |input| match input {
        Some(text) => text,
        None => (),
    };

    #[action("Append to the case value.")]
    let end = |&mut text| {
        text.push('!');
        text.len()
    };

    #[action("Use zero when absent.")]
    let end = |absent| 0;

    |end| return end;
}

#[test]
fn a_case_value_can_be_mutated_through_a_borrow() {
    assert_eq!(mutably_borrowed_choice_output(Some(String::from("hi"))), 3);
    assert_eq!(mutably_borrowed_choice_output(None), 0);
}
