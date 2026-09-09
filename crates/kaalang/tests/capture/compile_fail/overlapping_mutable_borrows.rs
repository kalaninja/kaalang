use kaalang::kaalang;

#[kaalang]
fn overlapping_mutable_borrows(mut text: String) -> usize {
    #[action("Keep a mutable reference.")]
    |&mut text| -> first { text };

    #[action("Borrow the wire again while its first reference is live.")]
    |&mut text, first| -> result {
        text.push('!');
        first.len()
    };
}

fn main() {}
