use kaalang::kaa;

#[kaa]
fn invalid(input: i32) -> &'static str {
    #[kaa::case("The input is negative.")]
    #[kaa::choice("What is the sign of the input?")]
    #[kaa::case("The input is nonnegative.")]
    let (negative, nonnegative) = |input| { negative };
}

fn main() {}
