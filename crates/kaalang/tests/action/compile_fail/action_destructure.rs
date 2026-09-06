use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Produce one value.")]
    |&input| -> (left, right) { *input };

    #[action("Add both outputs.")]
    |left, right| -> result { left + right };
}

fn main() {}
