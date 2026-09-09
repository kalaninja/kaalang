use kaalang::kaalang;

#[kaalang]
fn mutable_borrow_with_live_shared_reference(mut text: String) -> usize {
    #[action("Keep a shared reference.")]
    let view = |&text| { text.as_str() };

    #[action("Mutate while the shared reference remains live.")]
    let result = |&mut text, view| {
        text.push('!');
        view.len()
    };
}

fn main() {}
