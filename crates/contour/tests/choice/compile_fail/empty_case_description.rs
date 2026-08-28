use contour::contour;

#[contour]
fn invalid(input: i32) -> &'static str {
    #[choice("What is the sign of the input?")]
    #[case("")]
    #[case("The input is nonnegative.")]
    |input| -> (negative, nonnegative) { negative };
}

fn main() {}
