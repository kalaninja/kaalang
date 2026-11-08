use kaalang::kaa;

#[kaa]
fn invalid(condition: bool) -> u32 {
    #[kaa::question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Build the yes values.")]
    let (number, _log) = |yes| { (1, "yes") };

    #[kaa::action("Build the no values.")]
    let (number, _log) = |no| { (2, 3u8) };

    |number| return number;
}

fn main() {}
