use kaalang::kaa;

#[kaa]
fn invalid(condition: bool, value: u32) -> u32 {
    #[kaa::question("Choose a return.")]
    let (first, second) = |condition| condition;

    |first, value| return value;
    |second, value| return value;
}

fn main() {}
