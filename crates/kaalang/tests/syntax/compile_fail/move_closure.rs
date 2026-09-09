use kaalang::kaalang;

#[kaalang]
fn move_closure(value: i32) -> i32 {
    #[action("Double the value.")]
    let doubled = move |value| { value * 2 };
}

fn main() {}
