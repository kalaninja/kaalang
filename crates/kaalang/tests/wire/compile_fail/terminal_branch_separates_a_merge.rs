use kaalang::kaalang;

#[kaalang]
fn invalid(refine: bool, finish_early: bool) -> u32 {
    #[question("Refine the value?")]
    |refine| -> (nested, direct) { refine };

    #[question("Finish early?")]
    |nested, finish_early| -> (join, skip) { !finish_early };

    #[action("Finish without the shared step.")]
    |skip| -> result { 100 };

    #[action("Build the refined value.")]
    |join| -> shared { 1 };

    #[action("Build the direct value.")]
    |direct| -> shared { 2 };

    #[action("Add ten in the shared step.")]
    |shared| -> result { shared + 10 };
}

fn main() {}
