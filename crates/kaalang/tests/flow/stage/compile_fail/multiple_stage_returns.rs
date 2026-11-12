use kaalang::kaa;

#[kaa]
fn multiple_stage_returns(go: ()) {
    #[kaa::stage("Finish.")]
    |go| {
        return;
        return;
    };
}

fn main() {}
