use kaalang::kaa;

#[kaa]
fn stage_output_shadows_entry() {
    #[kaa::action("Go.")]
    let go = || ();
    #[kaa::stage("Go.")]
    |go| {
        #[kaa::action("Bad.")]
        let go = || ();
        return;
    };
}

fn main() {}
