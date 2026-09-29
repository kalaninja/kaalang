use kaalang::kaalang;

#[kaalang]
fn duplicate_stage_entry(go: ()) {
    #[stage("First.")]
    let next = |go| {
        #[action("Forward.")]
        let next = |go| ();
    };
    #[stage("Again.")]
    |go| {
        return;
    };
}

fn main() {}
