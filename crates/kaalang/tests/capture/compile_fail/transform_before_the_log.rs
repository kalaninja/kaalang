use kaalang::kaalang;

// The move is written above the borrow, so Rust reports the borrow.
#[kaalang]
fn invalid(value: String) -> usize {
    #[action("Transform the value into its length.")]
    let length = |value| { value.len() };

    #[action("Log the value.")]
    |&value| {
        assert_eq!(value, "abc");
    };

    #[action("Finish.")]
    let end = |length| { length };
}

fn main() {}
