use kaalang::kaalang;

#[kaalang(unexpected)]
fn kaalang_arguments(value: i32) -> i32 {
    #[action("Double the value.")]
    |value| -> doubled { value * 2 };
}

fn main() {}
