use kaalang::kaalang;

// The view borrows the text, and the length moves it while the view is still
// live. Rust reports the move; the flow is otherwise well formed.
#[kaalang]
fn invalid(condition: bool) -> usize {
    #[question("Build the text?")]
    let (yes, no) = |condition| { condition };

    #[action("Build text.")]
    let text = |yes| { String::from("hello") };

    #[action("Borrow text.")]
    let view = |&text| { text.as_str() };

    #[action("Measure the text.")]
    let length = |text| { text.len() };

    #[action("Log the view.")]
    |view| {
        assert_eq!(view, "hello");
    };

    #[action("Finish yes.")]
    let end = |length| { length };

    #[action("Finish no.")]
    let end = |no| { 0 };
}

fn main() {}
