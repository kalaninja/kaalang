use kaalang::kaalang;

#[kaalang]
fn duplicate_block_input(value: i32) -> i32 {
    #[action("Double the value.")]
    let result = |value, value| { value * 2 };
}

fn main() {}
