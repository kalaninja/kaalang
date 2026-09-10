use kaalang::kaalang;

#[kaalang]
fn immutable_merged_output(condition: bool) -> u32 {
    #[question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[action("Produce the first immutable alternative.")]
    let value = |yes| { 1 };

    #[action("Produce the second immutable alternative.")]
    let value = |no| { 2 };

    #[action("Try to mutate the merged wire.")]
    |&mut value| { *value += 1 };

    #[action("Return the value.")]
    let end = |value| { value };
}

fn main() {}
