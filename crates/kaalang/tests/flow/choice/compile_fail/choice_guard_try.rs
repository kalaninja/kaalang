use kaalang::kaa;

#[kaa]
fn invalid(value: u32) -> u32 {
    #[kaa::choice("Does the predecessor stay positive?")]
    #[kaa::case("The predecessor is positive.")]
    #[kaa::case("There is no positive predecessor.")]
    let (positive, other) = |value| {
        match value {
            current if current.checked_sub(1)? > 0 => current,
            _ => (),
        }
    };

    #[kaa::action("Produce the positive result.")]
    let result = |positive| positive;

    #[kaa::action("Produce the other result.")]
    let result = |other| { 0 };

    |result| return result;
}

fn main() {}
