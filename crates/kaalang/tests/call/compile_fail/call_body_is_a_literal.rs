use kaalang::kaa;

#[kaa]
fn invalid(value: u32) -> u32 {
    #[kaa::call("Produce one.")]
    let end = |value| 1;

    |end| return end;
}

fn main() {}
