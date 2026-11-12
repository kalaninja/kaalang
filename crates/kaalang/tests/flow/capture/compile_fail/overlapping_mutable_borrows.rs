use kaalang::kaa;

#[kaa]
fn overlapping_mutable_borrows(mut text: String) -> usize {
    #[kaa::action("Keep a mutable reference.")]
    let first = |&mut text| { text };

    #[kaa::action("Borrow the wire again while its first reference is live.")]
    let end = |&mut text, first| {
        text.push('!');
        first.len()
    };

    |end| return end;
}

fn main() {}
