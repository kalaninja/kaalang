use kaalang::kaalang;

// The other zero-output spelling: an omitted arrow declares no wires, so Rust
// checks the body against `()` just as `-> ()` does.
#[kaalang]
fn invalid(input: u32) {
    #[action("Omit the outputs and return a value anyway.")]
    |input| {
        let doubled = input * 2;
        doubled
    };

    #[action("Finish.")]
    || -> result {};
}

fn main() {}
