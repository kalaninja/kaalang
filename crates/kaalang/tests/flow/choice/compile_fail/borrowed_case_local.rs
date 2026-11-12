use kaalang::kaa;

// The selected case value leaves its arm before the branch continues, so it
// cannot borrow a binding that the arm owns.
#[kaa]
fn invalid(value: String) -> usize {
    #[kaa::choice("Borrow the matched string.")]
    #[kaa::case("Use a nonempty string.")]
    #[kaa::case("Use an empty string.")]
    let (nonempty, empty) = |value| {
        match value {
            owned if !owned.is_empty() => &owned,
            owned => &owned,
        }
    };

    #[kaa::action("Measure the nonempty string.")]
    let end = |nonempty| { nonempty.len() };

    #[kaa::action("Measure the empty string.")]
    let end = |empty| { empty.len() };

    |end| return end;
}

fn main() {}
