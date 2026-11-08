use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<u32>) {
    #[kaa::cycle("Visit every value.")]
    |values| for value in values {
        #[kaa::action("Use the value.")]
        let used = |value| drop(value);

        |used| continue;
    };

    return;
}

fn main() {}
