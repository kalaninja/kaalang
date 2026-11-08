use kaalang::kaa;

#[kaa]
fn mutable_stage_output_binding() {
    #[kaa::action("Go.")]
    let go = || ();
    #[kaa::stage("Go.")]
    let mut go = |go| {
        #[kaa::action("Again.")]
        let go = || ();
    };
}

fn main() {}
