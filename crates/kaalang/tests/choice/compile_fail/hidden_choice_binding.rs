use kaalang::kaalang;

#[kaalang]
fn invalid(input: Option<u32>) -> u32 {
    #[choice("Is a value present?")]
    #[case("A value is present.")]
    #[case("No value is present.")]
    |input| -> (present, absent) {
        match input {
            Some(hidden) => (),
            None => (),
        }
    };

    #[action("Produce the hidden match value.")]
    |present| -> result { hidden };

    #[action("Produce zero for no value.")]
    |absent| -> result { 0 };

    #[end]
    |result| {};
}

fn main() {}
