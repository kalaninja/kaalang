use kaalang::kaalang;

#[kaalang]
fn bracketed_description(value: i32) -> i32 {
    #[action["Double the value."]]
    let doubled = |value| { value * 2 };
}

fn main() {}
