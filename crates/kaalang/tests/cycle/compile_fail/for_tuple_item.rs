use kaalang::kaalang;

#[kaalang]
fn invalid(values: Vec<u32>) {
    #[cycle("Visit every value with its position.")]
    |values| for (index, value) in values.into_iter().enumerate() {
        #[action("Use the pair.")]
        |index, value| drop((index, value));
    };

    return;
}

fn main() {}
