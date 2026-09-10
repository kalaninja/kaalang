use kaalang::kaalang;

#[kaalang]
fn invalid(secret: u32, trigger: ()) -> (u32, u32) {
    #[action("Read an internal wire name.")]
    let output = |trigger| { __kaalang_wire_0 };

    #[action("Pair the two values.")]
    let end = |output, secret| { (output, secret) };
}

fn main() {}
