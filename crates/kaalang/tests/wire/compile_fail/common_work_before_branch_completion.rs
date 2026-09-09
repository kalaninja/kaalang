use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> u8 {
    #[question("Choose the value.")]
    let (yes, no) = |flag| { flag };

    #[action("Build the yes value and local work.")]
    let (value, local) = |yes| { (1u8, ()) };

    #[action("Build the no value.")]
    let value = |no| { 2u8 };

    #[action("Common effect.")]
    || {};

    #[action("Local effect.")]
    |local| {};

    #[action("Finish.")]
    let result = |value| { value };
}

fn main() {}
