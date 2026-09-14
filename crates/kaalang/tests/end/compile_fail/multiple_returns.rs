use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool, value: u32) -> u32 {
    #[question("Choose a return.")]
    let (first, second) = |condition| condition;

    |first, value| return value;
    |second, value| return value;
}

fn main() {}
