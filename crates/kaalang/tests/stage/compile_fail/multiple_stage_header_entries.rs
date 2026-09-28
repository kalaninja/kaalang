use kaalang::kaalang;

#[kaalang]
fn multiple_stage_header_entries(go: ()) {
    #[stage("Finish.")]
    |go, other| {
        return;
    };
}

fn main() {}
