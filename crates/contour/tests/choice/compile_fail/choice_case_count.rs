use contour::contour;

#[contour]
fn invalid(input: i32) -> &'static str {
    #[choice("What is the sign of the input?")]
    #[case("The input is negative.")]
    |input| -> (negative, nonnegative) {
        match input {
            ..0 => (),
            _ => (),
        }
    };

    #[action("Produce the negative result.")]
    |negative| -> negative_result { "negative" };

    #[action("Produce the nonnegative result.")]
    |nonnegative| -> nonnegative_result { "nonnegative" };
}

fn main() {}
