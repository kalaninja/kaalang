use kaalang::kaalang;

#[kaalang]
fn invalid(refine: bool, finish_early: bool) -> u32 {
    #[question("Refine the value?")]
    let (nested, direct) = |refine| { refine };

    #[question("Finish early?")]
    let (join, skip) = |nested, finish_early| { !finish_early };

    #[action("Finish without the shared step.")]
    let result = |skip| { 100 };

    #[action("Build the refined value.")]
    let shared = |join| { 1 };

    #[action("Build the direct value.")]
    let shared = |direct| { 2 };

    #[action("Add ten in the shared step.")]
    let result = |shared| { shared + 10 };
}

fn main() {}
