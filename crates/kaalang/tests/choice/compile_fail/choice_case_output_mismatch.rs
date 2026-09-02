use kaalang::kaalang;

#[kaalang]
fn invalid(input: i32) -> &'static str {
    #[choice("What is the sign of the input?")]
    #[case("The input is negative.")]
    #[case("The input is zero or positive.")]
    |input| -> (negative, zero, positive) {
        match input {
            ..0 => (),
            0 => (),
            _ => (),
        }
    };

    #[action("Produce the negative result.")]
    |negative| -> negative_result { "negative" };

    #[action("Produce the zero result.")]
    |zero| -> zero_result { "zero" };

    #[action("Produce the positive result.")]
    |positive| -> positive_result { "positive" };
}

fn main() {}
