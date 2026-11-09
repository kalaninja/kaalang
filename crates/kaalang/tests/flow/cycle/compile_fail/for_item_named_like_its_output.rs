use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<u32>) {
    #[kaa::cycle("Visit every value.")]
    let value = |values| for value in values {
        #[kaa::action("Use the value.")]
        |value| drop(value);
    };

    |value| return;
}

fn main() {}
