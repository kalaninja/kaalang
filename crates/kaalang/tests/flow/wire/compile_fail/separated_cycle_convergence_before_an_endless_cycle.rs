use kaalang::kaa;

#[kaa]
fn separated_cycle_convergence_before_an_endless_cycle(value: u8) -> ! {
    #[kaa::cycle("Settle once.")]
    let settled = loop {
        #[kaa::choice("Select a branch.")]
        #[kaa::case("Take the first branch.")]
        #[kaa::case("Skip the first merge.")]
        #[kaa::case("Take the second branch.")]
        let (first, skip, second) = |value| match value {
            0 => (),
            1 => (),
            _ => (),
        };

        #[kaa::action("Build the first value.")]
        let selected = |first| 1;

        #[kaa::action("Build the second value.")]
        let selected = |second| 3;

        #[kaa::action("Use the selected value.")]
        let settled = |selected| ();

        #[kaa::action("Skip the selected value.")]
        let settled = |skip| ();
    };

    #[kaa::cycle("Serve forever.")]
    |settled| loop {
        continue;
    };
}

fn main() {}
