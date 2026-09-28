use kaalang::kaalang;

#[kaalang]
fn simultaneous_stage_outputs(go: ()) {
    #[stage("Choose.")]
    let (next, finish) = |go| {
        #[action("Next.")]
        let next = |go| ();
        #[action("Finish.")]
        let finish = || ();
    };
    #[stage("Continue.")]
    let finish = |next| {
        #[action("Finish later.")]
        let finish = |next| ();
    };
    #[stage("Return.")]
    |finish| {
        return;
    };
}

fn main() {}
