use contour::contour;

#[contour]
fn run_question(condition: bool) -> &'static str {
    #[question("Take the yes branch?")]
    |condition| -> (yes, no) { condition };

    #[action("Return the yes result.")]
    |yes| -> result { "yes" };

    #[action("Return the no result.")]
    |no| -> result { "no" };

    #[end]
    |result| {};
}

#[test]
fn question_executes_each_branch() {
    assert_eq!(run_question(true), "yes");
    assert_eq!(run_question(false), "no");
}
