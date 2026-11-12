use kaalang::kaa;

#[kaa]
fn invalid(input: i32) {
    #[kaa::choice("What is the sign of the input?")]
    #[kaa::case("The input is negative.")]
    #[kaa::case("The input is nonnegative.")]
    let (negative, nonnegative) = |input| {
        match input {
            ..0 => (),
            _ => (),
        }
    };

    #[kaa::action("Handle only the negative input.")]
    let handled = |negative| { drop(negative) };

    |handled| return handled;
}

fn main() {}
