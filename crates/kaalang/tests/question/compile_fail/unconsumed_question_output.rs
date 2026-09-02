use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[question("Is the input large?")]
    |&input| -> (large, small) { *input > 10 };

    #[action("Handle only the large input.")]
    |large, &input| -> handled { *input };

    #[end]
    |handled| {};
}

fn main() {}
