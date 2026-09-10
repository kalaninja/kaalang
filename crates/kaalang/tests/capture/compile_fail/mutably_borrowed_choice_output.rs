use kaalang::kaalang;

#[kaalang]
fn mutably_borrowed_choice_output(input: Option<String>) -> usize {
    #[choice("Was text supplied?")]
    #[case("Text supplied.")]
    #[case("No text.")]
    let (mut text, absent) = |input| {
        match input {
            Some(text) => text,
            None => (),
        }
    };

    #[action("Try to borrow the case value.")]
    let end = |&mut text| {
        text.push('!');
        text.len()
    };

    #[action("Use zero when absent.")]
    let end = |absent| { 0 };
}

fn main() {}
