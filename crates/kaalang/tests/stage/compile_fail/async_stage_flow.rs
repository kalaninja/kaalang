use kaalang::kaalang;

#[kaalang]
async fn async_stage_flow(go: ()) {
    #[stage("Finish.")]
    |go| {
        return;
    };
}

fn main() {}
