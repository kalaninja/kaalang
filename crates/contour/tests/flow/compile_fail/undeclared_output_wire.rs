use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[action("Copy the input.")]
    |input| -> copied { input };

    #[action("Increment the copied value.")]
    |copied| -> incremented { copied + 1 };

    #[action("Read an output wire omitted from the inputs.")]
    |incremented| -> output { incremented + copied };
}

fn main() {}
