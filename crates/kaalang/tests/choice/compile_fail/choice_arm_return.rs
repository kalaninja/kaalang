use kaalang::kaalang;

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[choice("Is the value zero?")]
    #[case("The value is zero.")]
    #[case("The value is positive.")]
    let (zero, positive) = |value| {
        match value {
            0 => return 0,
            current => current,
        }
    };

    #[action("Produce the zero result.")]
    let end = |zero| { 0 };

    #[action("Produce the positive result.")]
    let end = |positive| { positive };
}

fn main() {}
