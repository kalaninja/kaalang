use kaalang::kaalang;

#[kaalang]
fn invalid(values: Vec<u32>, limit: usize) -> usize {
    #[cycle("Visit the first values.")]
    |values| for value in values.into_iter().take(limit) {
        #[action("Use the value.")]
        |value| drop(value);
    };

    |limit| return limit;
}

fn main() {}
