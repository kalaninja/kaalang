use kaalang::kaalang;

// A flow input named `result` finishes the flow before any block runs, so the
// only valid flow with one is the empty identity.
#[kaalang]
fn invalid(result: u32, other: u32) -> u32 {
    #[action("Work below the flow input.")]
    |other| -> _stray { other };
}

fn main() {}
