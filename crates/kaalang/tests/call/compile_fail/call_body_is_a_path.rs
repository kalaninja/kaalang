use kaalang::kaalang;

fn increment(value: u32) -> u32 {
    value + 1
}

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[call("Increment the value.")]
    let end = |value| increment;

    |end| return end;
}

fn main() {}
