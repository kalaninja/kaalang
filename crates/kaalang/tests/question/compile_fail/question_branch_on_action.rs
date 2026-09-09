use kaalang::kaalang;

#[kaalang]
fn invalid() -> u8 {
    #[action("Produce the result.")]
    #[yes]
    || -> result { 1 };
}

fn main() {}
