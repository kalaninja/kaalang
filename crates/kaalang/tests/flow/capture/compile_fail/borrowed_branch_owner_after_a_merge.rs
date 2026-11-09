use kaalang::kaa;

// The text belongs to the yes branch. Its reference cannot leave that scope
// through the view merge; Rust reports the dangling borrow.
#[kaa]
fn invalid(condition: bool) -> usize {
    #[kaa::question("Build the text?")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Build the owned text.")]
    let text = |yes| { String::from("abc") };

    #[kaa::action("Borrow the text.")]
    let view = |&text| { text.as_str() };

    #[kaa::action("Use the fallback text.")]
    let view = |no| { "fallback" };

    #[kaa::action("Measure the merged view.")]
    let end = |view| { view.len() };

    |end| return end;
}

fn main() {}
