use kaalang::kaa;

macro_rules! assign {
    ($value:ident) => {
        $value = 9;
    };
}

#[kaa]
fn uncaptured_parameter_macro(value: u32) -> u32 {
    #[kaa::action("Assign through an opaque macro without a capture.")]
    { assign!(value); };

    |value| return value;
}

fn main() {}
