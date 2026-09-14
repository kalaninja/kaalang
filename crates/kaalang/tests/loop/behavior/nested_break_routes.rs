use kaalang::kaalang;

#[kaalang]
fn nested_break_routes(a: bool, b: bool, c: bool) {
    #[cycle("Leave from any nested question.")]
    |a, b, c| {
        #[question("First?")]
        let (next, stop_a) = |a| a;
        #[question("Second?")]
        let (next_b, stop_b) = |next, b| b;
        #[question("Third?")]
        let (_again, stop_c) = |next_b, c| c;
        |stop_c| break;
        |stop_b| break;
        |stop_a| break;
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
