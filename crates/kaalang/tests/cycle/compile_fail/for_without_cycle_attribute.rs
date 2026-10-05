use kaalang::kaalang;

#[kaalang]
fn invalid(values: Vec<u32>) {
    #[action("Visit every value.")]
    for value in values {}

    return;
}

fn main() {}
