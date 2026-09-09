use kaalang::kaalang;

#[kaalang]
fn missing_let_semicolon(input: u32) -> u32 {
    #[action("Return the input.")]
    let result = |input| { input }
}

fn main() {}
