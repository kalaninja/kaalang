use kaalang::kaa;

fn increment(value: u32) -> u32 {
    value + 1
}

#[kaa]
fn invalid(value: u32) -> u32 {
    #[kaa::call("Increment the value.")]
    #[kaa::case("A case belongs to a choice.")]
    let end = |value| increment;

    |end| return end;
}

fn main() {}
