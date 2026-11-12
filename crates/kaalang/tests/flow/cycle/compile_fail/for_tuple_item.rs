use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<u32>) {
    #[kaa::cycle("Visit every value with its position.")]
    |values| for (index, value) in values.into_iter().enumerate() {
        #[kaa::action("Use the pair.")]
        |index, value| drop((index, value));
    };

    return;
}

fn main() {}
