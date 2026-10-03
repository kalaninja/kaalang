use kaalang::kaalang;

#[kaalang]
fn invalid(values: Vec<u32>) {
    #[cycle("Visit every value.")]
    |values| for value in values {
        #[action("Use the value.")]
        let used = |value| drop(value);

        |used| continue;
    };

    return;
}

fn main() {}
