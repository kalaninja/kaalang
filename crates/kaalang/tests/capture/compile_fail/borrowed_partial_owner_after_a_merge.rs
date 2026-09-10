use kaalang::kaalang;

// The inner merge owns the text. The wider merge ends its scope, so Rust
// rejects carrying a reference to that text into the shared continuation.
#[kaalang]
fn invalid(source: u8) -> usize {
    #[choice("Which source?")]
    #[case("First source.")]
    #[case("Second source.")]
    #[case("Fallback.")]
    let (first, second, fallback) = |source| {
        match source {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the first text.")]
    let text = |first| { String::from("abc") };

    #[action("Build the second text.")]
    let text = |second| { String::from("defg") };

    #[action("Borrow the partially merged text.")]
    let view = |&text| { text.as_str() };

    #[action("Use the fallback text.")]
    let view = |fallback| { "fallback" };

    #[action("Measure the merged view.")]
    let end = |view| { view.len() };
}

fn main() {}
