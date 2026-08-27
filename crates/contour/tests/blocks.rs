use contour::contour;

#[contour]
fn run_action(input: u32) -> u32 {
    #[action("Increment the input.")]
    |input| -> output { input + 1 };
}

#[contour]
fn run_question(condition: bool) -> &'static str {
    #[question("Take the yes branch?")]
    |condition| -> (yes, no) { condition };

    #[action("Return the yes result.")]
    |yes| -> yes_result { "yes" };

    #[action("Return the no result.")]
    |no| -> no_result { "no" };
}

#[test]
fn action_block_executes() {
    assert_eq!(run_action(1), 2);
}

#[test]
fn question_block_routes_both_outputs() {
    assert_eq!(run_question(true), "yes");
    assert_eq!(run_question(false), "no");
}
