use kaalang::kaalang;

#[kaalang]
fn invalid(value: i32) -> i32 {
    #[action("Return the doubled value instead of producing its wire.")]
    |value| -> doubled { return value * 2 };

    #[action("Produce the result.")]
    |doubled| -> result { doubled };
}

fn main() {}
