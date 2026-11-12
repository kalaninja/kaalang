use kaalang::kaa;

fn increment(value: u32) -> u32 {
    value + 1
}

#[kaa]
fn invalid(value: u32) -> u32 {
    #[kaa::call("Increment the value.")]
    let end = |value| increment(value + 1);

    |end| return end;
}

fn main() {}
