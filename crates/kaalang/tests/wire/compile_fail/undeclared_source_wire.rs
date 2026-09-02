use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32, trigger: ()) -> (u32, u32) {
    #[action("Read a source wire omitted from the inputs.")]
    |trigger| -> output { input };

    #[end]
    |output, input| {};
}

fn main() {}
