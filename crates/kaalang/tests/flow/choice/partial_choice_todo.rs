//! A lint of `unreachable!` sees none in the generated code.
#![forbid(clippy::unreachable)]

use kaalang::kaa;

#[kaa]
fn partial_choice_todo(input: i32) -> &'static str {
    #[kaa::choice("Is the input negative?")]
    #[kaa::case("The input is negative.")]
    #[kaa::case("The input is nonnegative.")]
    let (negative, nonnegative) = |input| match input {
        ..0 => todo!(),
        _ => (),
    };

    #[kaa::action("Produce the negative result.")]
    let result = |negative| "negative";

    #[kaa::action("Produce the nonnegative result.")]
    let result = |nonnegative| "nonnegative";

    |result| return result;
}

#[test]
fn a_written_arm_runs_beside_a_placeholder() {
    assert_eq!(partial_choice_todo(1), "nonnegative");
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn a_placeholder_arm_panics_when_selected() {
    partial_choice_todo(-1);
}
