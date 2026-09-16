use kaalang::kaalang;

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[action("Capture a receiver this flow does not declare.")]
    let end = |self, value| value;

    |end| return end;
}

fn main() {}
