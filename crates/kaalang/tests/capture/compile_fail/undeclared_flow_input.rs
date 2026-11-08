use kaalang::kaa;

#[kaa]
fn invalid(input: u32, trigger: ()) -> (u32, u32) {
    #[kaa::action("Read a flow input omitted from the inputs.")]
    let output = |trigger| { input };

    #[kaa::action("Pair the two values.")]
    let end = |output, input| { (output, input) };

    |end| return end;
}

fn main() {}
