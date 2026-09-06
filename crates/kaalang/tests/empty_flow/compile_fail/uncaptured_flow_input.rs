use kaalang::kaalang;

#[kaalang]
fn invalid(value: u8) {
    #[action("Finish without the flow input.")]
    || -> result {};
}

fn main() {}
