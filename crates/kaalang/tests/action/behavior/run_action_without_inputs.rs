use kaalang::kaalang;

#[kaalang]
fn run_action_without_inputs() -> u32 {
    #[action("Produce the answer without consuming a wire.")]
    || -> value { 42 };

    #[end]
    |value| {};
}

#[test]
fn an_action_without_inputs_is_ready_when_the_flow_begins() {
    assert_eq!(run_action_without_inputs(), 42);
}
