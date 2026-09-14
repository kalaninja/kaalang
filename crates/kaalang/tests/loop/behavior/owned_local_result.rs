use kaalang::kaalang;

#[kaalang]
fn owned_local_result(count: usize) -> String {
    #[cycle("Repeat before producing an owned local result.")]
    let result = |mut count| {
        #[question("Has the countdown finished?")]
        let (done, again) = |count| count == 0;

        #[action("Advance the countdown.")]
        |again, &mut count| *count -= 1;

        #[action("Build the owned result.")]
        let text = |done| String::from("done");

        |text| break text;
    };

    |result| return result;
}

#[test]
fn an_owned_iteration_local_can_move_out_after_repeats() {
    assert_eq!(owned_local_result(0), "done");
    assert_eq!(owned_local_result(3), "done");
}
