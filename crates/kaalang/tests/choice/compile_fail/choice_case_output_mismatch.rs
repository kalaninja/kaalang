use kaalang::kaa;

#[kaa]
fn invalid(input: i32) -> &'static str {
    #[kaa::choice("What is the sign of the input?")]
    #[kaa::case("The input is negative.")]
    #[kaa::case("The input is zero or positive.")]
    let (negative, zero, positive) = |input| {
        match input {
            ..0 => (),
            0 => (),
            _ => (),
        }
    };

    #[kaa::action("Produce the negative result.")]
    let negative_result = |negative| { "negative" };

    #[kaa::action("Produce the zero result.")]
    let zero_result = |zero| { "zero" };

    #[kaa::action("Produce the positive result.")]
    let positive_result = |positive| { "positive" };
}

fn main() {}
