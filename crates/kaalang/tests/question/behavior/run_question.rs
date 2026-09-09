use kaalang::kaalang;

#[kaalang]
fn run_question(condition: bool) -> &'static str {
    #[question("Take the yes branch?")]
    #[no("The condition is false.")]
    #[yes("The condition is true.")]
    |condition| -> (no, yes) { condition };

    #[action("Produce the no result.")]
    |no| -> result { "no" };

    #[action("Produce the yes result.")]
    |yes| -> result { "yes" };
}

#[test]
fn question_executes_each_branch() {
    assert_eq!(run_question(true), "yes");
    assert_eq!(run_question(false), "no");
}
