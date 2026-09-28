use kaalang::kaalang;

#[kaalang]
fn terminal_stage_not_last(go: ()) {
    #[stage("Finish.")]
    |finish| {
        return;
    };

    #[stage("Begin.")]
    let finish = |go| {
        #[action("Select the terminal stage.")]
        let finish = || ();
    };
}

fn main() {}
