use kaalang::kaalang;

fn increment(value: u32) -> u32 {
    value + 1
}

#[kaalang]
fn call_a_described_function(value: u32) -> u32 {
    #[call("Increment the value.")]
    let end = |value| increment(value);

    |end| return end;
}

#[test]
fn a_call_block_runs_its_function() {
    assert_eq!(call_a_described_function(1), 2);
}
