use kaalang::kaalang;

// `result` finishes an execution, so the stray work below it never runs.
#[kaalang]
fn invalid(input: u32, other: u32) -> u32 {
    #[action("Produce the result.")]
    |input| -> result { input };

    #[action("Work written below the result.")]
    |other| -> _stray { other };
}

fn main() {}
