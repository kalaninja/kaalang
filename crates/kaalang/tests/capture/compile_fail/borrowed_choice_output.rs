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
    let end = |&value| { *value * 2 };

    #[action("Produce the absent result.")]
    let end = |absent| { 0 };

    |end| return end;
}

fn main() {}
