use kaalang::kaa;

fn increment(value: u32) -> u32 {
    value + 1
}

#[kaa]
fn invalid(value: u32) -> u32 {
    #[kaa::call("Increment one.")]
    let end = |value| increment(1);

    |end| return end;
}

fn main() {}
