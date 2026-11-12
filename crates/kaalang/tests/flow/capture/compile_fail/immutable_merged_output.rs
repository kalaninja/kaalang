use kaalang::kaa;

#[kaa]
fn immutable_merged_output(condition: bool) -> u32 {
    #[kaa::question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Produce the first immutable alternative.")]
    let value = |yes| { 1 };

    #[kaa::action("Produce the second immutable alternative.")]
    let value = |no| { 2 };

    #[kaa::action("Try to mutate the merged wire.")]
    |&mut value| { *value += 1 };

    |value| return value;
}

fn main() {}
