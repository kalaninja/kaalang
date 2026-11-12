use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<u32>) {
    #[kaa::action("Visit every value.")]
    for value in values {}

    return;
}

fn main() {}
