use kaalang::kaa;

#[kaa]
fn terminal_stage_not_last(go: ()) {
    #[kaa::stage("Finish.")]
    |finish| {
        return;
    };

    #[kaa::stage("Begin.")]
    let finish = |go| {
        #[kaa::action("Select the terminal stage.")]
        let finish = || ();
    };
}

fn main() {}
