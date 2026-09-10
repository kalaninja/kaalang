use kaalang::kaalang;

#[kaalang]
fn immutable_output(input: String) -> String {
    #[action("Produce immutable text.")]
    let text = |input| { input };

    #[action("Try to borrow the output mutably.")]
    |&mut text| { text.clear() };

    #[action("Return the text.")]
    let end = |text| { text };
}

fn main() {}
