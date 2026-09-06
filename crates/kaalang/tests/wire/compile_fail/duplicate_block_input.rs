use kaalang::kaalang;

#[kaalang]
fn duplicate_block_input(value: i32) -> i32 {
    #[action("Double the value.")]
    |value, value| -> result { value * 2 };
}

fn main() {}
