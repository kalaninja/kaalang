use kaalang::kaalang;

#[kaalang]
fn inconsistent_output_mutability(condition: bool) -> u32 {
    #[question("Choose the producer.")]
    let (yes, no) = |condition| { condition };

    #[action("Produce a mutable unused wire.")]
    let (mut _value, end) = |yes| { (1, 1) };

    #[action("Produce an immutable alternative with the same logical name.")]
    let (r#_value, end) = |no| { (2, 2) };
}

fn main() {}
