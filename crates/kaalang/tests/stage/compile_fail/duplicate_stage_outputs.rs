use kaalang::kaa;

#[kaa]
fn duplicate_stage_outputs(go: ()) {
    #[kaa::stage("Repeat.")]
    let (go, go) = |go| {
        #[kaa::action("Repeat.")]
        let go = |go| ();
    };
}

fn main() {}
