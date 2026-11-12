use kaalang::kaa;

#[kaa]
fn simultaneous_initial_stage_signals() {
    #[kaa::action("A.")]
    let go = || ();
    #[kaa::action("B.")]
    let stop = || ();
    #[kaa::stage("Go.")]
    let go = |go| {
        #[kaa::action("Again.")]
        let go = |go| go;
    };
    #[kaa::stage("Stop.")]
    |stop| {
        return;
    };
}

fn main() {}
