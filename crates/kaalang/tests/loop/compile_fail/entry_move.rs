use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    #[action("Own the text.")]
    let text = |flag| flag.to_string();
    |text| loop {
        break;
    };
    #[action("Use the moved text.")]
    let end = |text| text.len();
}

fn main() {}
