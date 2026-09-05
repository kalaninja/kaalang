use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[choice("Choose without inputs.")]
    #[case("The first case.")]
    #[case("The second case.")]
    || -> (first, second) {
        match 0 {
            0 => (),
            _ => (),
        }
    };
}

fn main() {}
