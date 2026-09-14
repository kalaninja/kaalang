use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> String {
    |input| return input;
}

fn main() {}
