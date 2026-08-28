use contour::contour;

#[contour]
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

    #[action("Return the hidden match value.")]
    |present| -> present_result { hidden };

    #[action("Return zero for no value.")]
    |absent| -> absent_result { 0 };
}

fn main() {}
