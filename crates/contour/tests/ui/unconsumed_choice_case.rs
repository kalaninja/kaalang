use contour::contour;

#[contour]
fn invalid(input: i32) -> &'static str {
    #[choice("What is the sign of the input?")]
    #[case("The input is negative.")]
    #[case("The input is nonnegative.")]
    |input| -> (negative, nonnegative) {
        if input < 0 { negative } else { nonnegative }
    };

    #[action("Return only the negative result.")]
    |negative| -> negative_result { "negative" };
}

fn main() {}
