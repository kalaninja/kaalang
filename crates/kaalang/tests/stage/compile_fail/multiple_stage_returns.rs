use kaalang::kaalang;

#[kaalang]
fn multiple_stage_returns(go: ()) {
    #[stage("Finish.")]
    |go| {
        return;
        return;
    };
}

fn main() {}
