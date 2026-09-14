use kaalang::kaalang;

#[kaalang]
fn invalid(value: usize) {
    #[cycle("Complete with a value but no output wires.")]
    |value| {
        |value| break value;
    };

    return;
}

fn main() {}
