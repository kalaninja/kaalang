use kaalang::kaalang;

macro_rules! assign {
    ($value:ident) => {
        $value = 9;
    };
}

#[kaalang]
fn uncaptured_parameter_macro(value: u32) -> u32 {
    #[action("Assign through an opaque macro without a capture.")]
    { assign!(value); };

    |value| return value;
}

fn main() {}
