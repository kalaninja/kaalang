use kaalang::kaa;

#[kaa]
fn bracketed_description(value: i32) -> i32 {
    #[kaa::action["Double the value."]]
    let doubled = |value| { value * 2 };
}

fn main() {}
