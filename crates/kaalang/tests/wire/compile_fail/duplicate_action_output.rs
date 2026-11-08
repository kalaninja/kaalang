use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> (u32, u32) {
    #[kaa::action("Produce two outputs with one name.")]
    let (end, end) = |input| { (input, input) };

    |end| return end;
}

fn main() {}
