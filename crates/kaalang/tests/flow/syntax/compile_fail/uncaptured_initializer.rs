use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::action("Copy the input without capturing it.")]
    let end = input;

    |end, input| return end;
}

fn main() {}
