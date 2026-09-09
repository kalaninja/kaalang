use kaalang::kaalang;

#[kaalang]
fn invalid(input: i32) -> &'static str {
    #[choice("Is the input negative?")]
    #[case("The input is negative.")]
    #[case("The input is nonnegative.")]
    let (negative, nonnegative) = |input| {
        match input {
            ..0 => {
                todo!()
            }
            _ => (),
        }
    };

    #[action("Produce the negative result.")]
    let negative_result = |negative| { "negative" };

    #[action("Produce the nonnegative result.")]
    let nonnegative_result = |nonnegative| { "nonnegative" };
}

fn main() {}
