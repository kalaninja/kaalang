use kaalang::kaalang;

#[kaalang]
fn invalid(secret: u32, trigger: ()) -> (u32, u32) {
    #[action("Read an internal wire name.")]
    |trigger| -> output { __kaalang_wire_0 };

    #[action("Pair the two values.")]
    |output, secret| -> result { (output, secret) };
}

fn main() {}
