use kaalang::kaa;

#[kaa]
fn transition_after_wire_merge(flag: bool) {
    #[kaa::question("Select a branch.")]
    let (yes, no) = |flag| flag;

    #[kaa::action("Build the first signal.")]
    let (selected, go) = |yes| (1u8, ());

    #[kaa::action("Build the second signal.")]
    let (selected, done) = |no| (2u8, ());

    #[kaa::action("Use the merged value.")]
    |selected| {
        let _ = selected;
    };

    #[kaa::stage("Forward.")]
    let done = |go| {
        #[kaa::action("Select finish.")]
        let done = |go| ();
    };

    #[kaa::stage("Finish.")]
    |done| {
        return;
    };
}

fn main() {}
