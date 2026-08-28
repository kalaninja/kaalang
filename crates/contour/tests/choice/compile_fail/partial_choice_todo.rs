use contour::contour;

#[contour]
fn invalid(input: i32) -> &'static str {
    #[choice("Is the input negative?")]
    #[case("The input is negative.")]
    #[case("The input is nonnegative.")]
    |input| -> (negative, nonnegative) {
        match input {
            ..0 => {
                todo!()
            }
            _ => (),
        }
    };

    #[action("Return the negative result.")]
    |negative| -> negative_result { "negative" };

    #[action("Return the nonnegative result.")]
    |nonnegative| -> nonnegative_result { "nonnegative" };
}

fn main() {}
