use kaalang::kaalang;

#[kaalang]
fn duplicate_block_input(value: i32) -> i32 {
    #[action("Double the value.")]
    |value, value| -> doubled { value * 2 };

    #[end]
    |doubled| {};
}

fn main() {}
