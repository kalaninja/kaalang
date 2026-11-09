use kaalang::kaa;

#[kaa]
fn duplicate_mutable_capture(value: u8) -> u8 {
    #[kaa::action("Capture the same logical wire twice.")]
    let end = |mut value, &mut r#value| { *value };

    |end| return end;
}

fn main() {}
