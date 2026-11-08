use kaalang::kaa;

#[kaa]
fn mutable_stage_header_entry(go: ()) {
    #[kaa::stage("Finish.")]
    |mut go| {
        return;
    };
}

fn main() {}
