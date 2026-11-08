use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<u32>) {
    |values| for value in values {
        #[kaa::action("Use the value.")]
        |value| drop(value);
    };

    return;
}

fn main() {}
