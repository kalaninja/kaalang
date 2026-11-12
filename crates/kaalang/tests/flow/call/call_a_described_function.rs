use kaalang::kaa;

fn increment(value: u32) -> u32 {
    value + 1
}

#[kaa]
fn call_a_described_function(value: u32) -> u32 {
    #[kaa::call("Increment the value.")]
    let end = |value| increment(value);

    |end| return end;
}

#[test]
fn a_call_block_runs_its_function() {
    assert_eq!(call_a_described_function(1), 2);
}
