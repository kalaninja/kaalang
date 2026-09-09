use kaalang::kaalang;

#[kaalang]
fn overlapping_mutable_borrows(mut text: String) -> usize {
    #[action("Keep a mutable reference.")]
    let first = |&mut text| { text };

    #[action("Borrow the wire again while its first reference is live.")]
    let result = |&mut text, first| {
        text.push('!');
        first.len()
    };
}

fn main() {}
