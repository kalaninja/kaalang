use kaalang::kaalang;

// The view borrows the text, and the length moves it while the view is still
// live. Rust reports the move; the flow is otherwise well formed.
#[kaalang]
fn invalid(condition: bool) -> usize {
    #[question("Build the text?")]
    |condition| -> (yes, no) { condition };

    #[action("Build text.")]
    |yes| -> text { String::from("hello") };

    #[action("Borrow text.")]
    |&text| -> view { text.as_str() };

    #[action("Measure the text.")]
    |text| -> length { text.len() };

    #[action("Log the view.")]
    |view| {
        assert_eq!(view, "hello");
    };

    #[action("Finish yes.")]
    |length| -> result { length };

    #[action("Finish no.")]
    |no| -> result { 0 };
}

fn main() {}
