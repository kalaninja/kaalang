use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<u32>) {
    #[kaa::cycle("Visit every value.")]
    |values| 'values: for value in values {
        #[kaa::action("Use the value.")]
        |value| drop(value);
    };

    return;
}

fn main() {}
