use kaalang::kaalang;

#[kaalang]
fn mutable_parameter(mut value: i32) -> i32 {
    #[action("Double the value.")]
    |value| -> doubled { value * 2 };
}

fn main() {}
