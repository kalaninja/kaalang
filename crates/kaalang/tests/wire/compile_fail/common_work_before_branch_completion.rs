use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) -> u8 {
    #[kaa::question("Choose the value.")]
    let (yes, no) = |flag| { flag };

    #[kaa::action("Build the yes value and local work.")]
    let (value, local) = |yes| { (1, ()) };

    #[kaa::action("Build the no value.")]
    let value = |no| { 2 };

    #[kaa::action("Common effect.")]
    || {};

    #[kaa::action("Local effect.")]
    |local| {};

    |value| return value;
}

fn main() {}
