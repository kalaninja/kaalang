use kaalang::kaalang;

#[kaalang]
fn invalid(input: Option<u8>) -> u8 {
    #[choice("Was a value supplied?")]
    #[case("A value is available.")]
    #[case("No value is available.")]
    |input| -> (value, absent) {
        match input {
            Some(value) => value,
            None => (),
        }
    };

    #[action("Borrow the supplied value.")]
    |&value| -> result { *value * 2 };

    #[action("Produce the absent result.")]
    |absent| -> result { 0 };
}

fn main() {}
