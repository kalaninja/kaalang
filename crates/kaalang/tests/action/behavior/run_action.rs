use kaalang::kaalang;

#[kaalang]
fn run_action(input: u32) -> u32 {
    #[action("Increment the input.")]
    let end = |input| input + 1;

    |end| return end;
}

#[test]
fn action_block_executes() {
    assert_eq!(run_action(1), 2);
}
