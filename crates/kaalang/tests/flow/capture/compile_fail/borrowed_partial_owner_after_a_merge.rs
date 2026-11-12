use kaalang::kaa;

// The inner merge owns the text. The wider merge ends its scope, so Rust
// rejects carrying a reference to that text into the shared continuation.
#[kaa]
fn invalid(source: u8) -> usize {
    #[kaa::choice("Which source?")]
    #[kaa::case("First source.")]
    #[kaa::case("Second source.")]
    #[kaa::case("Fallback.")]
    let (first, second, fallback) = |source| {
        match source {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[kaa::action("Build the first text.")]
    let text = |first| { String::from("abc") };

    #[kaa::action("Build the second text.")]
    let text = |second| { String::from("defg") };

    #[kaa::action("Borrow the partially merged text.")]
    let view = |&text| { text.as_str() };

    #[kaa::action("Use the fallback text.")]
    let view = |fallback| { "fallback" };

    #[kaa::action("Measure the merged view.")]
    let end = |view| { view.len() };

    |end| return end;
}

fn main() {}
