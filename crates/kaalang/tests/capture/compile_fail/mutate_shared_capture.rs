use kaalang::kaalang;

#[kaalang]
fn mutate_shared_capture(text: String) -> usize {
    #[action("Try to mutate a shared reference.")]
    let end = |&text| {
        text.push('!');
        text.len()
    };
}

fn main() {}
