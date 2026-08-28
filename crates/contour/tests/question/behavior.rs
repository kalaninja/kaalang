use contour::contour;

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
fn question_block_routes_both_outputs() {
    assert_eq!(run_question(true), "yes");
    assert_eq!(run_question(false), "no");
}
