use kaalang::kaalang;

#[kaalang]
fn invalid(input: i32) {
    #[choice("What is the sign of the input?")]
    #[case("The input is negative.")]
    #[case("The input is nonnegative.")]
    |input| -> (negative, nonnegative) {
        match input {
            ..0 => (),
            _ => (),
        }
    };

    #[action("Handle only the negative input.")]
    |negative| -> () { drop(negative) };

    #[end]
    || {};
}

fn main() {}
