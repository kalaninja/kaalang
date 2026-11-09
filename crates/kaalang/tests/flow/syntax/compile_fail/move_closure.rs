use kaalang::kaa;

#[kaa]
fn move_closure(value: i32) -> i32 {
    #[kaa::action("Double the value.")]
    let doubled = move |value| { value * 2 };
}

fn main() {}
