use kaalang::kaa;

#[kaa]
fn nonterminal_stage_returns() {
    #[kaa::action("Go.")]
    let go = || ();
    #[kaa::stage("Go.")]
    let go = |go| {
        return;
    };
}

fn main() {}
