use kaalang::kaa;

#[kaa]
fn multiple_stage_header_entries(go: ()) {
    #[kaa::stage("Finish.")]
    |go, other| {
        return;
    };
}

fn main() {}
