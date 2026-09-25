use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Copy the input without capturing it.")]
    let end = input;

    |end, input| return end;
}

fn main() {}
