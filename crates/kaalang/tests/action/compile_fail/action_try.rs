use kaalang::kaalang;

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[action("Double the value, propagating overflow with `?`.")]
    |value| -> doubled {
        if value > 0 {
            value.checked_mul(2)?
        } else {
            0
        }
    };

    #[action("Produce the result.")]
    |doubled| -> result { doubled };

    #[end]
    |result| {};
}

fn main() {}
