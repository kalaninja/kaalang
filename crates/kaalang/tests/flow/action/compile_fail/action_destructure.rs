use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::action("Produce one value.")]
    let (left, right) = |&input| { *input };

    #[kaa::action("Add both outputs.")]
    let end = |left, right| { left + right };

    |end| return end;
}

fn main() {}
