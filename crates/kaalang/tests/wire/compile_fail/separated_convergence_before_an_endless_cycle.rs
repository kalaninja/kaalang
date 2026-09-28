use kaalang::kaalang;

#[kaalang]
fn separated_convergence_before_an_endless_cycle(value: u8) -> ! {
    #[choice("Select a branch.")]
    #[case("Take the first branch.")]
    #[case("Skip the first merge.")]
    #[case("Take the second branch.")]
    let (first, skip, second) = |value| match value {
        0 => (),
        1 => (),
        _ => (),
    };

    #[action("Build the first value.")]
    let selected = |first| 1;

    #[action("Build the second value.")]
    let selected = |second| 3;

    #[action("Use the selected value.")]
    let ready = |selected| ();

    #[action("Skip the selected value.")]
    let ready = |skip| ();

    #[cycle("Serve forever.")]
    |ready| {
        continue;
    };
}

fn main() {}
