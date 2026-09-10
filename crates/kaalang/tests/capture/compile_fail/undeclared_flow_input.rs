use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32, trigger: ()) -> (u32, u32) {
    #[action("Read a flow input omitted from the inputs.")]
    let output = |trigger| { input };

    #[action("Pair the two values.")]
    let end = |output, input| { (output, input) };
}

fn main() {}
