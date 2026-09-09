use kaalang::kaalang;

#[kaalang]
fn invalid(input: Option<u8>) -> u8 {
    #[choice("Was a value supplied?")]
    #[case("A value is available.")]
    #[case("No value is available.")]
    let (value, absent) = |input| {
        match input {
            Some(value) => value,
            None => (),
        }
    };

    #[action("Borrow the supplied value.")]
    let result = |&value| { *value * 2 };

    #[action("Produce the absent result.")]
    let result = |absent| { 0 };
}

fn main() {}
