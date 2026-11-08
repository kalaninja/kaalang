use kaalang::kaa;

// The move is written above the borrow, so Rust reports the borrow.
#[kaa]
fn invalid(value: String) -> usize {
    #[kaa::action("Transform the value into its length.")]
    let length = |value| { value.len() };

    #[kaa::action("Log the value.")]
    |&value| {
        assert_eq!(value, "abc");
    };

    |length| return length;
}

fn main() {}
