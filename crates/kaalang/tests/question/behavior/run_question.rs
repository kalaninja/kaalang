use kaalang::kaalang;

#[kaalang]
fn run_question(condition: bool) -> &'static str {
    #[question("Take the yes branch?")]
    |condition| -> (yes, no) { condition };

    #[action("Produce the yes result.")]
    |yes| -> result { "yes" };

    #[action("Produce the no result.")]
    |no| -> result { "no" };
}

#[test]
fn question_executes_each_branch() {
    assert_eq!(run_question(true), "yes");
    assert_eq!(run_question(false), "no");
}
