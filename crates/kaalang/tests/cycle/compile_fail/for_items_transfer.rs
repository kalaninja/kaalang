use kaalang::kaalang;

#[kaalang]
fn invalid(values: Vec<Vec<u32>>) -> Option<u32> {
    #[cycle("Visit the values of the first row.")]
    |values| for value in values.first()? {
        #[action("Use the value.")]
        |value| drop(value);
    };

    return None;
}

fn main() {}
