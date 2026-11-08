use kaalang::kaa;

#[kaa(unexpected)]
fn kaalang_arguments(value: i32) -> i32 {
    #[kaa::action("Double the value.")]
    let doubled = |value| { value * 2 };
}

fn main() {}
