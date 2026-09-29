use kaalang::kaalang;

#[kaalang]
fn duplicate_stage_outputs(go: ()) {
    #[stage("Repeat.")]
    let (go, go) = |go| {
        #[action("Repeat.")]
        let go = |go| ();
    };
}

fn main() {}
