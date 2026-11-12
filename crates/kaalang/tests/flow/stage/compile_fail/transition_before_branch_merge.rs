use kaalang::kaa;

#[kaa]
fn transition_before_branch_merge(go: (), flag: bool) {
    #[kaa::question("Select a branch.")]
    let (yes, no) = |flag| flag;

    #[kaa::stage("Finish.")]
    |go| {
        return;
    };
}

fn main() {}
