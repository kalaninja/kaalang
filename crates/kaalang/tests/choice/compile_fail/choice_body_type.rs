use kaalang::kaalang;

#[kaalang]
fn invalid(input: i32) -> &'static str {
    #[choice("What is the sign of the input?")]
    #[case("The input is negative.")]
    #[case("The input is nonnegative.")]
    let (negative, nonnegative) = |input| { true };

    #[action("Produce the negative result.")]
    let negative_result = |negative| { "negative" };

    #[action("Produce the nonnegative result.")]
    let nonnegative_result = |nonnegative| { "nonnegative" };
}

fn main() {}
