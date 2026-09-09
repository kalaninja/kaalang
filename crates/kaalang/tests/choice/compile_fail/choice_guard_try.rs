use kaalang::kaalang;

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[choice("Does the predecessor stay positive?")]
    #[case("The predecessor is positive.")]
    #[case("There is no positive predecessor.")]
    let (positive, other) = |value| {
        match value {
            current if current.checked_sub(1)? > 0 => current,
            _ => (),
        }
    };

    #[action("Produce the positive result.")]
    let result = |positive| { positive };

    #[action("Produce the other result.")]
    let result = |other| { 0 };
}

fn main() {}
