use kaalang::kaalang;

#[kaalang]
fn stage_without_header(go: ()) {
    #[stage("Finish.")]
    {
        return;
    };
}

fn main() {}
