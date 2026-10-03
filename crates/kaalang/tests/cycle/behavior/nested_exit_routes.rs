use kaalang::kaalang;

#[kaalang]
fn nested_exit_routes(a: bool, b: bool, c: bool) {
    #[cycle("Leave from any nested question.")]
    let stop = loop {
        #[question("First?")]
        let (next, stop) = |a| a;
        #[question("Second?")]
        let (next_b, stop) = |next, b| b;
        #[question("Third?")]
        let (again, stop) = |next_b, c| c;
        |again| continue;
    };

    |stop| return;
}

#[test]
fn each_nested_question_can_leave_the_cycle() {
    for (a, b, c) in [
        (false, true, true),
        (true, false, true),
        (true, true, false),
    ] {
        nested_exit_routes(a, b, c);
    }
}
