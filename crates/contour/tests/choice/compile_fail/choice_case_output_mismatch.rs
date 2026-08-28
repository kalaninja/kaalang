use contour::contour;

#[contour]
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

    #[action("Return the negative result.")]
    |negative| -> negative_result { "negative" };

    #[action("Return the zero result.")]
    |zero| -> zero_result { "zero" };

    #[action("Return the positive result.")]
    |positive| -> positive_result { "positive" };
}

fn main() {}
