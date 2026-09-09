use kaalang::kaalang;

#[kaalang]
fn invalid() -> u8 {
    #[action("Produce the result.")]
    #[yes]
    let result = || { 1 };
}

fn main() {}
