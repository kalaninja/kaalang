use kaalang::kaalang;

#[kaalang]
fn transition_after_wire_merge(flag: bool) {
    #[question("Select a branch.")]
    let (yes, no) = |flag| flag;

    #[action("Build the first signal.")]
    let (selected, go) = |yes| (1u8, ());

    #[action("Build the second signal.")]
    let (selected, done) = |no| (2u8, ());

    #[action("Use the merged value.")]
    |selected| {
        let _ = selected;
    };

    #[stage("Forward.")]
    let done = |go| {
        #[action("Select finish.")]
        let done = |go| ();
    };

    #[stage("Finish.")]
    |done| {
        return;
    };
}

fn main() {}
