use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<u32>, limit: usize) -> usize {
    #[kaa::cycle("Visit the first values.")]
    |values| for value in values.into_iter().take(limit) {
        #[kaa::action("Use the value.")]
        |value| drop(value);
    };

    |limit| return limit;
}

fn main() {}
