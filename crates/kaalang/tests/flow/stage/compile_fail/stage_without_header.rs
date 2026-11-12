use kaalang::kaa;

#[kaa]
fn stage_without_header(go: ()) {
    #[kaa::stage("Finish.")]
    {
        return;
    };
}

fn main() {}
