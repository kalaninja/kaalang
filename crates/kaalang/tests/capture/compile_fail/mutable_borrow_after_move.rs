use kaalang::kaalang;

#[kaalang]
fn mutable_borrow_after_move(mut text: String) -> usize {
    #[action("Move the text into a mutable local.")]
    let moved = |mut text| {
        text.push('!');
        text
    };

    #[action("Try to mutate the moved wire.")]
    let result = |&mut text, moved| {
        text.push('?');
        moved.len()
    };
}

fn main() {}
