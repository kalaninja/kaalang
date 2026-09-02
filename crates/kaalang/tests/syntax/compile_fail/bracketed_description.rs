use kaalang::kaalang;

#[kaalang]
fn bracketed_description(value: i32) -> i32 {
    #[action["Double the value."]]
    |value| -> doubled { value * 2 };
}

fn main() {}
