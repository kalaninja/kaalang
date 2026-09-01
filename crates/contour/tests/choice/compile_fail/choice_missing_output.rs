use contour::contour;

#[contour]
fn invalid(input: i32) -> &'static str {
    #[choice("What is the sign of the input?")]
    #[case("The input is negative.")]
    #[case("The input is zero.")]
    #[case("The input is positive.")]
    |input| -> (negative, zero, positive) {
        match input {
            ..0 => (),
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
