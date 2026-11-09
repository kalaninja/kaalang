use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<u32>) {
    #[kaa::cycle("Visit every value.")]
    let done = |values| for value in values {
        #[kaa::action("Finish early.")]
        let done = |value| drop(value);
    };

    |done| return;
}

fn main() {}
