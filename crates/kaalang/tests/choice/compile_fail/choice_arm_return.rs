use kaalang::kaa;

#[kaa]
fn invalid(value: u32) -> u32 {
    #[kaa::choice("Is the value zero?")]
    #[kaa::case("The value is zero.")]
    #[kaa::case("The value is positive.")]
    let (zero, positive) = |value| {
        match value {
            0 => return 0,
            current => current,
        }
    };

    #[kaa::action("Produce the zero result.")]
    let result = |zero| { 0 };

    #[kaa::action("Produce the positive result.")]
    let result = |positive| positive;

    |result| return result;
}

fn main() {}
