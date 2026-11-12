use kaalang::kaa;

#[kaa]
fn stage_output_without_destination() {
    #[kaa::action("Go.")]
    let go = || ();
    #[kaa::stage("Go.")]
    let nowhere = |go| {
        #[kaa::action("Nowhere.")]
        let nowhere = || ();
    };
}

fn main() {}
