use kaalang::kaalang;

// The inner merge owns the text. The wider merge ends its scope, so Rust
// rejects carrying a reference to that text into the shared continuation.
#[kaalang]
fn invalid(source: u8) -> usize {
    #[choice("Which source?")]
    #[case("First source.")]
    #[case("Second source.")]
    #[case("Fallback.")]
    |source| -> (first, second, fallback) {
        match source {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the first text.")]
    |first| -> text { String::from("abc") };

    #[action("Build the second text.")]
    |second| -> text { String::from("defg") };

    #[action("Borrow the partially merged text.")]
    |&text| -> view { text.as_str() };

    #[action("Use the fallback text.")]
    |fallback| -> view { "fallback" };

    #[action("Measure the merged view.")]
    |view| -> result { view.len() };
}

fn main() {}
