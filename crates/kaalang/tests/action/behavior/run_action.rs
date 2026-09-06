use kaalang::kaalang;

#[kaalang]
fn run_action(input: u32) -> u32 {
    #[action("Increment the input.")]
    |input| -> result { input + 1 };
}

#[test]
fn action_block_executes() {
    assert_eq!(run_action(1), 2);
}
