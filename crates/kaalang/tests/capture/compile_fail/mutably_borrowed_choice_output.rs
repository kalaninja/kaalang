use kaalang::kaalang;

#[kaalang]
fn mutably_borrowed_choice_output(input: Option<String>) -> usize {
    #[choice("Was text supplied?")]
    #[case("Text supplied.")]
    #[case("No text.")]
    |input| -> (text, absent) {
        match input {
            Some(text) => text,
            None => (),
        }
    };

    #[action("Try to borrow the case value.")]
    |&mut text| -> result {
        text.push('!');
        text.len()
    };

    #[action("Use zero when absent.")]
    |absent| -> result { 0 };
}

fn main() {}
