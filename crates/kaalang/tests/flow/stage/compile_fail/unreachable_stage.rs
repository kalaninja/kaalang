use kaalang::kaa;

#[kaa]
fn unreachable_stage() {
    #[kaa::action("Go.")]
    let go = || ();
    #[kaa::stage("Lost.")]
    let lost = |lost| {
        #[kaa::action("Again.")]
        let lost = |lost| lost;
    };
    #[kaa::stage("Go.")]
    |go| {
        return;
    };
}

fn main() {}
