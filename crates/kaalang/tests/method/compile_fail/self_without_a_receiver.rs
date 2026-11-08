use kaalang::kaa;

#[kaa]
fn invalid(value: u32) -> u32 {
    #[kaa::action("Capture a receiver this flow does not declare.")]
    let end = |self, value| value;

    |end| return end;
}

fn main() {}
