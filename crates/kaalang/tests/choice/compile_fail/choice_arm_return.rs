use kaalang::kaalang;

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[choice("Is the value zero?")]
    #[case("The value is zero.")]
    #[case("The value is positive.")]
    |value| -> (zero, positive) {
        match value {
            0 => return 0,
            current => current,
        }
    };

    #[action("Produce the zero result.")]
    |zero| -> result { 0 };

    #[action("Produce the positive result.")]
    |positive| -> result { positive };
}

fn main() {}
