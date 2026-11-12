use kaalang::kaa;

#[kaa]
fn invalid(input: i32) -> &'static str {
    #[kaa::choice("Is the input negative?")]
    #[kaa::case("The input is negative.")]
    #[kaa::case("The input is nonnegative.")]
    let (negative, nonnegative) = |input| {
        match input {
            ..0 => {
                todo!()
            }
            _ => (),
        }
    };

    #[kaa::action("Produce the negative result.")]
    let negative_result = |negative| { "negative" };

    #[kaa::action("Produce the nonnegative result.")]
    let nonnegative_result = |nonnegative| { "nonnegative" };
}

fn main() {}
