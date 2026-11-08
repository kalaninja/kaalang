use kaalang::kaa;

#[kaa]
fn mutable_borrow_after_move(mut text: String) -> usize {
    #[kaa::action("Move the text into a mutable local.")]
    let moved = |mut text| {
        text.push('!');
        text
    };

    #[kaa::action("Try to mutate the moved wire.")]
    let end = |&mut text, moved| {
        text.push('?');
        moved.len()
    };

    |end| return end;
}

fn main() {}
