use kaalang::kaalang;

// The text belongs to the yes branch. Its reference cannot leave that scope
// through the view merge; Rust reports the dangling borrow.
#[kaalang]
fn invalid(condition: bool) -> usize {
    #[question("Build the text?")]
    let (yes, no) = |condition| { condition };

    #[action("Build the owned text.")]
    let text = |yes| { String::from("abc") };

    #[action("Borrow the text.")]
    let view = |&text| { text.as_str() };

    #[action("Use the fallback text.")]
    let view = |no| { "fallback" };

    #[action("Measure the merged view.")]
    let end = |view| { view.len() };
}

fn main() {}
