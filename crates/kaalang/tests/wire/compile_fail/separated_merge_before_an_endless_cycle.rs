use kaalang::kaalang;

#[kaalang]
fn separated_merge_before_an_endless_cycle(outer: bool, inner: bool) -> ! {
    #[question("Take the left side?")]
    let (left, right) = |outer| outer;

    #[question("Take the first branch?")]
    let (first, skip) = |left, inner| inner;

    #[action("Build the first value.")]
    let selected = |first| 1;

    #[action("Build the second value.")]
    let selected = |right| 3;

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
