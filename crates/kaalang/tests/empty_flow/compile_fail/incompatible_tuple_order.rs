use kaalang::kaalang;

#[kaalang]
fn invalid(number: u8, text: &'static str) -> (u8, &'static str) {
    #[end]
    |text, number| {};
}

fn main() {}
