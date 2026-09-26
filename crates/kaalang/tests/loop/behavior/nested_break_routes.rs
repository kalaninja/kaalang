use kaalang::kaalang;

#[kaalang]
fn nested_break_routes(a: bool, b: bool, c: bool) {
    #[cycle("Leave from any nested question.")]
    {
        #[question("First?")]
        let (next, stop) = |a| a;
        #[question("Second?")]
        let (next_b, stop) = |next, b| b;
        #[question("Third?")]
        let (again, stop) = |next_b, c| c;
        |stop| break;
        |again| continue;
    };

    return;
}

#[test]
fn each_nested_question_can_leave_the_loop() {
    for (a, b, c) in [
        (false, true, true),
        (true, false, true),
        (true, true, false),
    ] {
        nested_break_routes(a, b, c);
    }
}
