use kaalang::kaalang;

// The selected case value leaves its arm before the branch continues, so it
// cannot borrow a binding that the arm owns.
#[kaalang]
fn invalid(value: String) -> usize {
    #[choice("Borrow the matched string.")]
    #[case("Use a nonempty string.")]
    #[case("Use an empty string.")]
    |value| -> (nonempty, empty) {
        match value {
            owned if !owned.is_empty() => &owned,
            owned => &owned,
        }
    };

    #[action("Measure the nonempty string.")]
    |nonempty| -> result { nonempty.len() };

    #[action("Measure the empty string.")]
    |empty| -> result { empty.len() };

    #[end]
    |result| {};
}

fn main() {}
