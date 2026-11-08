use kaalang::kaa;

#[kaa]
fn borrowed_stage_header_entry(go: ()) {
    #[kaa::stage("Finish.")]
    |&go| {
        return;
    };
}

fn main() {}
