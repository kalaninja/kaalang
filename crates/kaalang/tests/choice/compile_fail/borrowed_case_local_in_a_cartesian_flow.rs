use kaalang::kaalang;

// The guarded schedule stores each case value in its output slot, so a case
// value cannot borrow a match binding there either.
#[kaalang]
fn invalid(value: String, left: bool) -> usize {
    #[choice("Borrow the matched string.")]
    #[case("Use a nonempty string.")]
    #[case("Use an empty string.")]
    |value| -> (nonempty, empty) {
        match value {
            owned if !owned.is_empty() => &owned,
            owned => &owned,
        }
    };

    #[question("Take the left path?")]
    |left| -> (a, b) { left };

    #[action("Measure the nonempty string on the left path.")]
    |nonempty, a| -> result { nonempty.len() };

    #[action("Measure the nonempty string on the right path.")]
    |nonempty, b| -> result { nonempty.len() + 10 };

    #[action("Measure the empty string on the left path.")]
    |empty, a| -> result { empty.len() };

    #[action("Measure the empty string on the right path.")]
    |empty, b| -> result { empty.len() + 10 };

    #[end]
    |result| {};
}

fn main() {}
