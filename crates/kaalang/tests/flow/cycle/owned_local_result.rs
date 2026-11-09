use kaalang::kaa;

#[kaa]
fn owned_local_result(mut count: usize) -> String {
    #[kaa::cycle("Repeat before producing an owned local result.")]
    let text = loop {
        #[kaa::question("Has the countdown finished?")]
        let (done, again) = |count| count == 0;

        #[kaa::action("Advance the countdown.")]
        |again, &mut count| *count -= 1;

        #[kaa::action("Build the owned result.")]
        let text = |done| String::from("done");

        |again| continue;
    };

    |text| return text;
}

#[test]
fn an_owned_iteration_local_can_move_out_after_repeats() {
    assert_eq!(owned_local_result(0), "done");
    assert_eq!(owned_local_result(3), "done");
}
