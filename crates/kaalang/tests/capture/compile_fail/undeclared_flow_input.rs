use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32, trigger: ()) -> (u32, u32) {
    #[action("Read a flow input omitted from the inputs.")]
    |trigger| -> output { input };

    #[action("Pair the two values.")]
    |output, input| -> result { (output, input) };
}

fn main() {}
