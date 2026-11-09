use kaalang::kaa;

#[kaa]
fn invalid(secret: u32, trigger: ()) -> (u32, u32) {
    #[kaa::action("Read an internal wire name.")]
    let output = |trigger| { __kaalang_wire_0 };

    #[kaa::action("Pair the two values.")]
    let end = |output, secret| { (output, secret) };

    |end| return end;
}

fn main() {}
