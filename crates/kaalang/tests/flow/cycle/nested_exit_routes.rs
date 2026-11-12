use kaalang::kaa;

#[kaa]
fn nested_exit_routes(a: bool, b: bool, c: bool) {
    #[kaa::cycle("Leave from any nested question.")]
    let stop = loop {
        #[kaa::question("First?")]
        let (next, stop) = |a| a;
        #[kaa::question("Second?")]
        let (next_b, stop) = |next, b| b;
        #[kaa::question("Third?")]
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
