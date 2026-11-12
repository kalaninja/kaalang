use kaalang::kaa;

#[kaa]
fn missing_let_semicolon(input: u32) -> u32 {
    #[kaa::action("Return the input.")]
    let end = |input| { input }
}

fn main() {}
