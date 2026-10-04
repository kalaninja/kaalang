use kaalang::kaalang;

#[kaalang]
fn invalid(values: &[u32]) {
    #[cycle("Visit every value.")]
    |values| for &value in values {
        #[action("Use the value.")]
        |value| drop(value);
    };

    return;
}

fn main() {}
