use kaalang::kaalang;

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[choice("Does the predecessor stay positive?")]
    #[case("The predecessor is positive.")]
    #[case("There is no positive predecessor.")]
    |value| -> (positive, other) {
        match value {
            current if current.checked_sub(1)? > 0 => current,
            _ => (),
        }
    };

    #[action("Produce the positive result.")]
    |positive| -> result { positive };

    #[action("Produce the other result.")]
    |other| -> result { 0 };

    #[end]
    |result| {};
}

fn main() {}
