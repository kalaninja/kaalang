use kaalang::kaalang;

#[kaalang]
fn unreachable_stage() {
    #[action("Go.")]
    let go = || ();
    #[stage("Lost.")]
    let lost = |lost| {
        #[action("Again.")]
        let lost = |lost| lost;
    };
    #[stage("Go.")]
    |go| {
        return;
    };
}

fn main() {}
