use kaalang::kaalang;

#[kaalang]
fn invalid() -> u8 {
    #[action("Produce a unit result.")]
    || -> result {};
}

fn main() {}
