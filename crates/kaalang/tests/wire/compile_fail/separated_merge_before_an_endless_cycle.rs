use kaalang::kaa;

#[kaa]
fn separated_merge_before_an_endless_cycle(outer: bool, inner: bool) -> ! {
    #[kaa::question("Take the left side?")]
    let (left, right) = |outer| outer;

    #[kaa::question("Take the first branch?")]
    let (first, skip) = |left, inner| inner;

    #[kaa::action("Build the first value.")]
    let selected = |first| 1;

    #[kaa::action("Build the second value.")]
    let selected = |right| 3;

    #[kaa::action("Use the selected value.")]
    let ready = |selected| ();

    #[kaa::action("Skip the selected value.")]
    let ready = |skip| ();

    #[kaa::cycle("Serve forever.")]
    |ready| loop {
        continue;
    };
}

fn main() {}
