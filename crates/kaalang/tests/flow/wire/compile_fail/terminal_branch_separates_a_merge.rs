use kaalang::kaa;

#[kaa]
fn invalid(refine: bool, finish_early: bool) -> u32 {
    #[kaa::question("Refine the value?")]
    let (nested, direct) = |refine| { refine };

    #[kaa::question("Finish early?")]
    let (join, skip) = |nested, finish_early| { !finish_early };

    #[kaa::action("Finish without the shared step.")]
    let end = |skip| { 100 };

    #[kaa::action("Build the refined value.")]
    let shared = |join| { 1 };

    #[kaa::action("Build the direct value.")]
    let shared = |direct| { 2 };

    #[kaa::action("Add ten in the shared step.")]
    let end = |shared| { shared + 10 };

    |end| return end;
}

fn main() {}
