use kaalang::kaa;

#[kaa]
fn stage_output_without_producer() {
    #[kaa::action("Go.")]
    let go = || ();
    #[kaa::stage("Go.")]
    let go = |go| {};
}

fn main() {}
