use kaalang::kaalang;

#[kaalang]
fn invalid() -> u8 {
    #[action("Produce the result.")]
    #[yes]
    let end = || { 1 };

    |end| return end;
}

fn main() {}
