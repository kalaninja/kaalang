use kaalang::kaalang;

#[kaalang]
fn mutate_shared_capture(text: String) -> usize {
    #[action("Try to mutate a shared reference.")]
    |&text| -> result {
        text.push('!');
        text.len()
    };
}

fn main() {}
