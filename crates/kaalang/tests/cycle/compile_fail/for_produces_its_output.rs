use kaalang::kaalang;

#[kaalang]
fn invalid(values: Vec<u32>) {
    #[cycle("Visit every value.")]
    let done = |values| for value in values {
        #[action("Finish early.")]
        let done = |value| drop(value);
    };

    |done| return;
}

fn main() {}
