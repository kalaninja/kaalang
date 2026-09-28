use kaalang::kaalang;

#[kaalang]
fn stage_output_without_producer() {
    #[action("Go.")]
    let go = || ();
    #[stage("Go.")]
    let go = |go| {};
}

fn main() {}
