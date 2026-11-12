use kaalang::kaa;

#[kaa]
fn invalid(value: u8) -> u8 {
    #[kaa::choice("Select a branch.")]
    #[kaa::case("Take the first branch.")]
    #[kaa::case("Finish without converging.")]
    #[kaa::case("Take the second branch.")]
    let (first, done, second) = |value| {
        match value {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[kaa::action("Build the first value.")]
    let selected = |first| { 1 };

    #[kaa::action("Produce the direct result.")]
    let direct = |done| { 2 };

    #[kaa::action("Build the second value.")]
    let selected = |second| { 3 };

    #[kaa::action("Use the selected value.")]
    let result = |selected| selected;

    #[kaa::action("Use the direct value.")]
    let result = |direct| direct;

    |result| return result;
}

fn main() {}
