use kaalang::kaalang;

fn increment(value: u32) -> u32 {
    value + 1
}

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[call("Increment one.")]
    let end = |value| increment(1);

    |end| return end;
}

fn main() {}
