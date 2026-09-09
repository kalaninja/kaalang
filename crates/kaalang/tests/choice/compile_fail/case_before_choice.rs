use kaalang::kaalang;

#[kaalang]
fn invalid(input: i32) -> &'static str {
    #[case("The input is negative.")]
    #[choice("What is the sign of the input?")]
    #[case("The input is nonnegative.")]
    let (negative, nonnegative) = |input| { negative };
}

fn main() {}
