use kaalang::kaalang;

// The text belongs to the yes branch. Its reference cannot leave that scope
// through the view merge; Rust reports the dangling borrow.
#[kaalang]
fn invalid(condition: bool) -> usize {
    #[question("Build the text?")]
    |condition| -> (yes, no) { condition };

    #[action("Build the owned text.")]
    |yes| -> text { String::from("abc") };

    #[action("Borrow the text.")]
    |&text| -> view { text.as_str() };

    #[action("Use the fallback text.")]
    |no| -> view { "fallback" };

    #[action("Measure the merged view.")]
    |view| -> result { view.len() };
}

fn main() {}
