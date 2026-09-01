use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[question("Return a number?")]
    |&input| -> (yes, no) { *input };

    #[action("Return the input from the yes branch.")]
    |yes, &input| -> result { *input };

    #[action("Return the input from the no branch.")]
    |no, &input| -> result { *input };

    #[end]
    |result| {};
}

fn main() {}
