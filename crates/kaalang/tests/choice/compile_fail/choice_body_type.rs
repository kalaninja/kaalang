use kaalang::kaalang;

#[kaalang]
fn invalid(input: i32) -> &'static str {
    #[choice("What is the sign of the input?")]
    #[case("The input is negative.")]
    #[case("The input is nonnegative.")]
    |input| -> (negative, nonnegative) { true };

    #[action("Produce the negative result.")]
    |negative| -> negative_result { "negative" };

    #[action("Produce the nonnegative result.")]
    |nonnegative| -> nonnegative_result { "nonnegative" };
}

fn main() {}
