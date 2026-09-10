use kaalang::kaalang;

// A flow input named `end` finishes the flow before any block runs, so the
// only valid flow with one is the empty identity.
#[kaalang]
fn invalid(end: u32, other: u32) -> u32 {
    #[action("Work below the flow input.")]
    let _stray = |other| { other };
}

fn main() {}
