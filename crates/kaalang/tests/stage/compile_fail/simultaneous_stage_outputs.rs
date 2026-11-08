use kaalang::kaa;

#[kaa]
fn simultaneous_stage_outputs(go: ()) {
    #[kaa::stage("Choose.")]
    let (next, finish) = |go| {
        #[kaa::action("Next.")]
        let next = |go| ();
        #[kaa::action("Finish.")]
        let finish = || ();
    };
    #[kaa::stage("Continue.")]
    let finish = |next| {
        #[kaa::action("Finish later.")]
        let finish = |next| ();
    };
    #[kaa::stage("Return.")]
    |finish| {
        return;
    };
}

fn main() {}
