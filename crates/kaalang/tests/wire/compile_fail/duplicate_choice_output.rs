use kaalang::kaalang;

#[kaalang]
fn invalid(value: u32) -> u32 {
    #[choice("Declare one name for both cases.")]
    #[case("Zero")]
    #[case("Nonzero")]
    |value| -> (branch, branch) {
        match value {
            0 => 0,
            _ => value,
        }
    };

    #[end]
    |branch| {};
}

fn main() {}
