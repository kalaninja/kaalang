use kaalang::kaalang;

#[kaalang]
fn nonterminal_stage_returns() {
    #[action("Go.")]
    let go = || ();
    #[stage("Go.")]
    let go = |go| {
        return;
    };
}

fn main() {}
