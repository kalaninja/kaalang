use kaalang::kaalang;

#[kaalang]
fn stage_without_transition_or_return() {
    #[action("Go.")]
    let go = || ();
    #[stage("Go.")]
    |go| {
        #[action("Again.")]
        let local = || ();
    };
}

fn main() {}
