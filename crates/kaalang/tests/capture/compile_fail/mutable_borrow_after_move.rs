use kaalang::kaalang;

#[kaalang]
fn mutable_borrow_after_move(mut text: String) -> usize {
    #[action("Move the text into a mutable local.")]
    |mut text| -> moved {
        text.push('!');
        text
    };

    #[action("Try to mutate the moved wire.")]
    |&mut text, moved| -> result {
        text.push('?');
        moved.len()
    };
}

fn main() {}
