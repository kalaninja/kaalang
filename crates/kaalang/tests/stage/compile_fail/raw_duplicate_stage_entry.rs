use kaalang::kaa;

#[kaa]
fn raw_duplicate_stage_entry(go: ()) {
    #[kaa::stage("First.")]
    let next = |go| {
        #[kaa::action("Forward.")]
        let next = |go| ();
    };
    #[kaa::stage("Again.")]
    |r#go| {
        return;
    };
}

fn main() {}
