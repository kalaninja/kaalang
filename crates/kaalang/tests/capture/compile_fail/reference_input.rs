use kaalang::kaa;

#[kaa]
fn reference_input(value: i32) -> i32 {
    #[kaa::action("Double the value.")]
    let doubled = |ref value| { *value * 2 };
}

fn main() {}
