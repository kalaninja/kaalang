use kaalang::kaa;

#[kaa]
fn invalid(value: i32) -> i32 {
    #[kaa::action("Return the doubled value instead of producing its wire.")]
    let doubled = |value| { return value * 2 };

    |doubled| return doubled;
}

fn main() {}
