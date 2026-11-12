use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<u32>, name: String) {
    #[kaa::cycle("Visit every value.")]
    |values| for value in values {
        #[kaa::action("Use the value and the name.")]
        |value, name| drop((value, name));
    };

    return;
}

fn main() {}
