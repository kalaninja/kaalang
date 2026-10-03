use kaalang::kaalang;

#[kaalang]
fn invalid(values: Vec<u32>) {
    #[cycle("Visit every value.")]
    let (found, done) = |values| for value in values {
        #[action("Use the value.")]
        |value| drop(value);
    };

    |done| return;
}

fn main() {}
