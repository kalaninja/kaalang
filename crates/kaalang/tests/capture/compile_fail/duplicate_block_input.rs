use kaalang::kaalang;

#[kaalang]
fn duplicate_block_input(value: i32) -> i32 {
    #[action("Double the value.")]
    let end = |value, value| { value * 2 };

    |end| return end;
}

fn main() {}
