use kaalang::kaalang;

#[kaalang]
fn raw_duplicate_stage_entry(go: ()) {
    #[stage("First.")]
    let next = |go| {
        #[action("Forward.")]
        let next = |go| ();
    };
    #[stage("Again.")]
    |r#go| {
        return;
    };
}

fn main() {}
