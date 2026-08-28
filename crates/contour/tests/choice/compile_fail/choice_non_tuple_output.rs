use contour::contour;

#[contour]
fn invalid(input: i32) -> &'static str {
    #[choice("Is the input negative?")]
    #[case("The input is negative.")]
    #[case("The input is nonnegative.")]
    |input| -> negative {
        match input {
            ..0 => (),
            _ => (),
        }
    };
}

fn main() {}
