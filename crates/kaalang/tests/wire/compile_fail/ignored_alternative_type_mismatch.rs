use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[action("Build the yes values.")]
    let (number, _log) = |yes| { (1, "yes") };

    #[action("Build the no values.")]
    let (number, _log) = |no| { (2, 3u8) };

    #[action("Use the number.")]
    let result = |number| { number };
}

fn main() {}
