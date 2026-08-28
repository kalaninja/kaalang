use contour::contour;

#[contour]
fn invalid(input: i32) -> &'static str {
    #[choice("What is the sign of the input?")]
    #[case("The input is negative.")]
    |input| -> (negative, nonnegative) { negative };

    #[action("Return the negative result.")]
    |negative| -> negative_result { "negative" };

    #[action("Return the nonnegative result.")]
    |nonnegative| -> nonnegative_result { "nonnegative" };
}

fn main() {}
