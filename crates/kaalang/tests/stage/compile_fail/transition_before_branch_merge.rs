use kaalang::kaalang;

#[kaalang]
fn transition_before_branch_merge(go: (), flag: bool) {
    #[question("Select a branch.")]
    let (yes, no) = |flag| flag;

    #[stage("Finish.")]
    |go| {
        return;
    };
}

fn main() {}
