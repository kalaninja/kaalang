use kaalang::kaalang;

#[kaalang]
fn mutable_borrow_with_live_shared_reference(mut text: String) -> usize {
    #[action("Keep a shared reference.")]
    |&text| -> view { text.as_str() };

    #[action("Mutate while the shared reference remains live.")]
    |&mut text, view| -> result {
        text.push('!');
        view.len()
    };
}

fn main() {}
