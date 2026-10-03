use kaalang::kaalang;

#[kaalang]
fn invalid(values: Vec<u32>, name: String) {
    #[cycle("Visit every value.")]
    |values| for value in values {
        #[action("Use the value and the name.")]
        |value, name| drop((value, name));
    };

    return;
}

fn main() {}
