use kaalang::kaa;

#[kaa]
fn invalid() {
    #[kaa::action("Create the values.")]
    let values = || vec![1, 2, 3];

    #[kaa::cycle("Visit the indices.")]
    |values| {
        for _ in 0..values.len() {
            #[kaa::action("Read an alias whose header scope has ended.")]
            || assert!(!values.is_empty());
        }
    };

    return;
}

fn main() {}
