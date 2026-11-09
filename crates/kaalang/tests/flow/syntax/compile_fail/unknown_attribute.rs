use kaalang::kaa;

#[kaa]
fn unknown_attribute(value: i32) -> i32 {
    #[kaa::action("Double the value.")]
    #[unknown("Not a kaalang attribute.")]
    let doubled = |value| { value * 2 };
}

fn main() {}
