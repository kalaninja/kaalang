use kaalang::kaalang;

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[choice("Is the successor zero?")]
    #[case("The successor is zero.")]
    #[case("The successor is positive.")]
    |value| -> (zero, positive) {
        match value.checked_add(1)? {
            0 => (),
            successor => successor,
        }
    };

    #[action("Produce the zero result.")]
    |zero| -> result { 0 };

    #[action("Produce the positive result.")]
    |positive| -> result { positive };
}

fn main() {}
