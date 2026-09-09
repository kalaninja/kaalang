use kaalang::kaalang;

#[kaalang]
fn mutable_reference_binding(value: u8) -> u8 {
    #[action("Try to modify both the reference and its binding.")]
    |&mut mut value| -> result { *value };
}

fn main() {}
