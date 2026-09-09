use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> u8 {
    #[question("Choose the value.")]
    |flag| -> (yes, no) { flag };

    #[action("Build the yes value and local work.")]
    |yes| -> (value, local) { (1u8, ()) };

    #[action("Build the no value.")]
    |no| -> value { 2u8 };

    #[action("Common effect.")]
    || {};

    #[action("Local effect.")]
    |local| {};

    #[action("Finish.")]
    |value| -> result { value };
}

fn main() {}
