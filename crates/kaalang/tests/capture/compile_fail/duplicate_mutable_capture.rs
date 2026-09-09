use kaalang::kaalang;

#[kaalang]
fn duplicate_mutable_capture(value: u8) -> u8 {
    #[action("Capture the same logical wire twice.")]
    let result = |mut value, &mut r#value| { *value };
}

fn main() {}
