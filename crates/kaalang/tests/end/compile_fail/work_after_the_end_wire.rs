use kaalang::kaalang;

// `end` finishes an execution, so the stray work below it never runs.
#[kaalang]
fn invalid(input: u32, other: u32) -> u32 {
    #[action("Produce the result.")]
    let end = |input| { input };

    #[action("Work written below the result.")]
    let _stray = |other| { other };
}

fn main() {}
