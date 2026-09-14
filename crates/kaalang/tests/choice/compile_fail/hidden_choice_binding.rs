use kaalang::kaalang;

#[kaalang]
fn invalid(input: Option<u32>) -> u32 {
    #[choice("Is a value present?")]
    #[case("A value is present.")]
    #[case("No value is present.")]
    let (present, absent) = |input| {
        match input {
            Some(hidden) => (),
            None => (),
        }
    };

    #[action("Produce the hidden match value.")]
    let end = |present| { hidden };

    #[action("Produce zero for no value.")]
    let end = |absent| { 0 };

    |end| return end;
}

fn main() {}
