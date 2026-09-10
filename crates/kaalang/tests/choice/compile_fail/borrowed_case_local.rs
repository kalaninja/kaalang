use kaalang::kaalang;

// The selected case value leaves its arm before the branch continues, so it
// cannot borrow a binding that the arm owns.
#[kaalang]
fn invalid(value: String) -> usize {
    #[choice("Borrow the matched string.")]
    #[case("Use a nonempty string.")]
    #[case("Use an empty string.")]
    let (nonempty, empty) = |value| {
        match value {
            owned if !owned.is_empty() => &owned,
            owned => &owned,
        }
    };

    #[action("Measure the nonempty string.")]
    let end = |nonempty| { nonempty.len() };

    #[action("Measure the empty string.")]
    let end = |empty| { empty.len() };
}

fn main() {}
