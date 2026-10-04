use kaalang::kaalang;

#[kaalang]
fn invalid(values: Vec<u32>) {
    |values| for value in values {
        #[action("Use the value.")]
        |value| drop(value);
    };

    return;
}

fn main() {}
