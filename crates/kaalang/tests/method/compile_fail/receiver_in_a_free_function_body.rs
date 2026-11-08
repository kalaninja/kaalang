use kaalang::kaa;

#[kaa]
fn invalid(value: u32) -> u32 {
    #[kaa::action("Read a receiver this flow does not declare.")]
    let end = |value| value + self.width;

    |end| return end;
}

fn main() {}
