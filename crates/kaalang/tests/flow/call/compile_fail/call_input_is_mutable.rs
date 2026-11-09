use kaalang::kaa;

fn increment(value: u32) -> u32 {
    value + 1
}

#[kaa]
fn invalid(value: u32) -> u32 {
    #[kaa::call("Increment the value.")]
    let end = |mut value| increment(value);

    |end| return end;
}

fn main() {}
