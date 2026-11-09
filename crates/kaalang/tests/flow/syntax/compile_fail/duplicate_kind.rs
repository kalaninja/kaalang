use kaalang::kaa;

#[kaa]
fn duplicate_kind(value: i32) -> i32 {
    #[kaa::action("Double the value.")]
    #[kaa::question("Is the value large?")]
    let doubled = |value| { value * 2 };
}

fn main() {}
