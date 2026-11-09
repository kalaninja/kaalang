use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<u32>) {
    #[kaa::cycle("Visit every value.")]
    let (found, done) = |values| for value in values {
        #[kaa::action("Use the value.")]
        |value| drop(value);
    };

    |done| return;
}

fn main() {}
