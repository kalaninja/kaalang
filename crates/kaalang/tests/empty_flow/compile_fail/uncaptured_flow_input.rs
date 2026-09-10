use kaalang::kaalang;

#[kaalang]
fn invalid(value: u8) {
    #[action("Finish without the flow input.")]
    let end = || {};
}

fn main() {}
