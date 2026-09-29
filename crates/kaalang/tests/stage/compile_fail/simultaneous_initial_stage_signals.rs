use kaalang::kaalang;

#[kaalang]
fn simultaneous_initial_stage_signals() {
    #[action("A.")]
    let go = || ();
    #[action("B.")]
    let stop = || ();
    #[stage("Go.")]
    let go = |go| {
        #[action("Again.")]
        let go = |go| go;
    };
    #[stage("Stop.")]
    |stop| {
        return;
    };
}

fn main() {}
