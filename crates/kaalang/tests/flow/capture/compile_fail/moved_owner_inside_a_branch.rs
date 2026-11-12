use kaalang::kaa;

// The view borrows the text, and the length moves it while the view is still
// live. Rust reports the move; the flow is otherwise well formed.
#[kaa]
fn invalid(condition: bool) -> usize {
    #[kaa::question("Build the text?")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Build text.")]
    let text = |yes| { String::from("hello") };

    #[kaa::action("Borrow text.")]
    let view = |&text| { text.as_str() };

    #[kaa::action("Measure the text.")]
    let result = |text| { text.len() };

    #[kaa::action("Log the view.")]
    |view| {
        assert_eq!(view, "hello");
    };

    #[kaa::action("Finish no.")]
    let result = |no| { 0 };

    |result| return result;
}

fn main() {}
