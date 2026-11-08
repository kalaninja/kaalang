use kaalang::kaa;

#[kaa]
fn invalid(input: Option<u32>) -> u32 {
    #[kaa::choice("Is a value present?")]
    #[kaa::case("A value is present.")]
    #[kaa::case("No value is present.")]
    let (present, absent) = |input| {
        match input {
            Some(hidden) => (),
            None => (),
        }
    };

    #[kaa::action("Produce the hidden match value.")]
    let end = |present| { hidden };

    #[kaa::action("Produce zero for no value.")]
    let end = |absent| { 0 };

    |end| return end;
}

fn main() {}
