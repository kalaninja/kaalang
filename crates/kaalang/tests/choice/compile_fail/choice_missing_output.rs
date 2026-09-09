use kaalang::kaalang;

#[kaalang]
fn invalid(input: i32) -> &'static str {
    #[choice("What is the sign of the input?")]
    #[case("The input is negative.")]
    #[case("The input is zero.")]
    #[case("The input is positive.")]
    let (negative, zero, positive) = |input| {
        match input {
            ..0 => (),
            _ => (),
        }
    };

    #[action("Produce the negative result.")]
    let negative_result = |negative| { "negative" };

    #[action("Produce the zero result.")]
    let zero_result = |zero| { "zero" };

    #[action("Produce the positive result.")]
    let positive_result = |positive| { "positive" };
}

fn main() {}
