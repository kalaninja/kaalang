use kaalang::kaalang;

#[kaalang]
fn invalid(value: i32) -> i32 {
    #[action("Return the doubled value instead of producing its wire.")]
    let doubled = |value| { return value * 2 };

    #[action("Produce the result.")]
    let end = |doubled| { doubled };
}

fn main() {}
