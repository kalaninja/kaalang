use kaalang::kaalang;

#[kaalang]
fn mutable_reference_binding(value: u8) -> u8 {
    #[action("Try to modify both the reference and its binding.")]
    let end = |&mut mut value| { *value };

    |end| return end;
}

fn main() {}
