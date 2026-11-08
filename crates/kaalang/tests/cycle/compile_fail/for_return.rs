use kaalang::kaa;

#[kaa]
fn invalid(values: &[u32]) -> u32 {
    #[kaa::cycle("Visit every value.")]
    |values| for value in values {
        |value| return *value;
    };

    return 0;
}

fn main() {}
