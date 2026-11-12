use kaalang::kaa;

#[kaa]
fn inconsistent_output_mutability(condition: bool) -> u32 {
    #[kaa::question("Choose the producer.")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Produce a mutable unused wire.")]
    let (mut _value, end) = |yes| { (1, 1) };

    #[kaa::action("Produce an immutable alternative with the same logical name.")]
    let (r#_value, end) = |no| { (2, 2) };

    |end| return end;
}

fn main() {}
