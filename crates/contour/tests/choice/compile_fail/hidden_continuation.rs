use contour::contour;

#[contour]
fn invalid(input: i32) -> &'static str {
    #[choice("Is the input negative?")]
    #[case("The input is negative.")]
    #[case("The input is nonnegative.")]
    |input| -> (negative, nonnegative) {
        match {
            __contour_continue_0_0!(());
            input
        } {
            ..0 => (),
            _ => (),
        }
    };

    #[action("Produce the negative result.")]
    |negative| -> result { "negative" };

    #[action("Produce the nonnegative result.")]
    |nonnegative| -> result { "nonnegative" };

    #[end]
    |result| {};
}

fn main() {}
