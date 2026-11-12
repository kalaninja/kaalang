use kaalang::kaa;

#[kaa]
fn run_question(condition: bool) -> &'static str {
    #[kaa::question("Take the yes branch?")]
    #[kaa::no("The condition is false.")]
    #[kaa::yes("The condition is true.")]
    let (no, yes) = |condition| condition;

    #[kaa::action("Produce the no result.")]
    let end = |no| "no";

    #[kaa::action("Produce the yes result.")]
    let end = |yes| "yes";

    |end| return end;
}

#[test]
fn question_executes_each_branch() {
    assert_eq!(run_question(true), "yes");
    assert_eq!(run_question(false), "no");
}
