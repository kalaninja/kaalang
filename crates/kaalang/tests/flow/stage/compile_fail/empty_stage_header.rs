use kaalang::kaa;

#[kaa]
fn empty_stage_header(go: ()) {
    #[kaa::stage("Finish.")]
    || {
        return;
    };
}

fn main() {}
