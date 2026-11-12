use kaalang::kaa;

#[kaa]
fn immutable_output(input: String) -> String {
    #[kaa::action("Produce immutable text.")]
    let text = |input| { input };

    #[kaa::action("Try to borrow the output mutably.")]
    |&mut text| { text.clear() };

    |text| return text;
}

fn main() {}
