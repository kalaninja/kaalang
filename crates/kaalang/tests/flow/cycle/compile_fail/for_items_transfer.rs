use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<Vec<u32>>) -> Option<u32> {
    #[kaa::cycle("Visit the values of the first row.")]
    |values| for value in values.first()? {
        #[kaa::action("Use the value.")]
        |value| drop(value);
    };

    return None;
}

fn main() {}
