use kaalang::kaa;

#[kaa]
fn invalid(input: i32) -> &'static str {
    #[kaa::choice("What is the sign of the input?")]
    #[kaa::case("The input is negative.")]
    let (negative, nonnegative) = |input| {
        match input {
            ..0 => (),
            _ => (),
        }
    };

    #[kaa::action("Produce the negative result.")]
    let negative_result = |negative| { "negative" };

    #[kaa::action("Produce the nonnegative result.")]
    let nonnegative_result = |nonnegative| { "nonnegative" };
}

fn main() {}
