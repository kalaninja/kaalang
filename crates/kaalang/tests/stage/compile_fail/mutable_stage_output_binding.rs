use kaalang::kaalang;

#[kaalang]
fn mutable_stage_output_binding() {
    #[action("Go.")]
    let go = || ();
    #[stage("Go.")]
    let mut go = |go| {
        #[action("Again.")]
        let go = || ();
    };
}

fn main() {}
