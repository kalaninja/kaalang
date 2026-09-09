use kaalang::kaalang;

#[kaalang]
fn duplicate_kind(value: i32) -> i32 {
    #[action("Double the value.")]
    #[question("Is the value large?")]
    let doubled = |value| { value * 2 };
}

fn main() {}
