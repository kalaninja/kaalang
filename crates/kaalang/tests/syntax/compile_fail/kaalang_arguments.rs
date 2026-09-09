use kaalang::kaalang;

#[kaalang(unexpected)]
fn kaalang_arguments(value: i32) -> i32 {
    #[action("Double the value.")]
    let doubled = |value| { value * 2 };
}

fn main() {}
