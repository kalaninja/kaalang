use kaalang::kaalang;

#[kaalang]
fn stage_output_shadows_entry() {
    #[action("Go.")]
    let go = || ();
    #[stage("Go.")]
    |go| {
        #[action("Bad.")]
        let go = || ();
        return;
    };
}

fn main() {}
