use kaalang::kaa;

// An action without an output declaration produces no wires, so Rust
// checks the body against `()` just as `let ()` does.
#[kaa]
fn invalid(input: u32) {
    #[kaa::action("Omit the outputs and return a value anyway.")]
    |input| {
        let doubled = input * 2;
        doubled
    };

    #[kaa::action("Finish.")]
    let end = || {};

    |end| return end;
}

fn main() {}
