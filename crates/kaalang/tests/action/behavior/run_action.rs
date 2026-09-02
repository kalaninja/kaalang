use kaalang::kaalang;

#[kaalang]
fn run_action(input: u32) -> u32 {
    #[action("Increment the input.")]
    |input| -> output { input + 1 };

    #[end]
    |output| {};
}

#[test]
fn action_block_executes() {
    assert_eq!(run_action(1), 2);
}
