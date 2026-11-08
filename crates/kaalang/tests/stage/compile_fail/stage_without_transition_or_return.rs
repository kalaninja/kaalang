use kaalang::kaa;

#[kaa]
fn stage_without_transition_or_return() {
    #[kaa::action("Go.")]
    let go = || ();
    #[kaa::stage("Go.")]
    |go| {
        #[kaa::action("Again.")]
        let local = || ();
    };
}

fn main() {}
