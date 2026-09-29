use kaalang::kaalang;

#[kaalang]
fn empty_stage_header(go: ()) {
    #[stage("Finish.")]
    || {
        return;
    };
}

fn main() {}
