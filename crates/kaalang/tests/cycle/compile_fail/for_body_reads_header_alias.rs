use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[action("Create the values.")]
    let values = || vec![1, 2, 3];

    #[cycle("Visit the indices.")]
    |values| {
        for _ in 0..values.len() {
            #[action("Read an alias whose header scope has ended.")]
            || assert!(!values.is_empty());
        }
    };

    return;
}

fn main() {}
