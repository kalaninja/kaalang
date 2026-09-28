use kaalang::kaalang;

#[kaalang]
fn stage_output_without_destination() {
    #[action("Go.")]
    let go = || ();
    #[stage("Go.")]
    let nowhere = |go| {
        #[action("Nowhere.")]
        let nowhere = || ();
    };
}

fn main() {}
