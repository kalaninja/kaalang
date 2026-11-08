use kaalang::kaa;

#[kaa]
async fn async_stage_flow(go: ()) {
    #[kaa::stage("Finish.")]
    |go| {
        return;
    };
}

fn main() {}
