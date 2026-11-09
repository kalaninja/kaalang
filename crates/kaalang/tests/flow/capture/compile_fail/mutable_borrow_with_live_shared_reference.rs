use kaalang::kaa;

#[kaa]
fn mutable_borrow_with_live_shared_reference(mut text: String) -> usize {
    #[kaa::action("Keep a shared reference.")]
    let view = |&text| { text.as_str() };

    #[kaa::action("Mutate while the shared reference remains live.")]
    let end = |&mut text, view| {
        text.push('!');
        view.len()
    };

    |end| return end;
}

fn main() {}
