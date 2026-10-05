use kaalang::kaalang;

#[kaalang]
fn invalid(values: Vec<u32>) {
    #[cycle("Visit every value.")]
    let value = |values| for value in values {
        #[action("Use the value.")]
        |value| drop(value);
    };

    |value| return;
}

fn main() {}
