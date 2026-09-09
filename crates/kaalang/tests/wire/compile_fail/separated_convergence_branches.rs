use kaalang::kaalang;

#[kaalang]
fn invalid(value: u8) -> u8 {
    #[choice("Select a branch.")]
    #[case("Take the first branch.")]
    #[case("Finish without converging.")]
    #[case("Take the second branch.")]
    let (first, done, second) = |value| {
        match value {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the first value.")]
    let selected = |first| { 1 };

    #[action("Produce the direct result.")]
    let result = |done| { 2 };

    #[action("Build the second value.")]
    let selected = |second| { 3 };

    #[action("Produce the selected result.")]
    let result = |selected| { selected };
}

fn main() {}
