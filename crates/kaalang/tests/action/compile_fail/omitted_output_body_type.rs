use kaalang::kaalang;

// An action without an output declaration produces no wires, so Rust
// checks the body against `()` just as `let ()` does.
#[kaalang]
fn invalid(input: u32) {
    #[action("Omit the outputs and return a value anyway.")]
    |input| {
        let doubled = input * 2;
        doubled
    };

    #[action("Finish.")]
    let end = || {};

    |end| return end;
}

fn main() {}
