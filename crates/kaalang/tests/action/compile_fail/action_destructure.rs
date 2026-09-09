use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Produce one value.")]
    let (left, right) = |&input| { *input };

    #[action("Add both outputs.")]
    let result = |left, right| { left + right };
}

fn main() {}
