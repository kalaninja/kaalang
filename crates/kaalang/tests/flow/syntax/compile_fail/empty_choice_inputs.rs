use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::choice("Choose without inputs.")]
    #[kaa::case("The first case.")]
    #[kaa::case("The second case.")]
    let (first, second) = || {
        match 0 {
            0 => (),
            _ => (),
        }
    };
}

fn main() {}
