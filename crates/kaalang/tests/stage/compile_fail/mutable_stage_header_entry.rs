use kaalang::kaalang;

#[kaalang]
fn mutable_stage_header_entry(go: ()) {
    #[stage("Finish.")]
    |mut go| {
        return;
    };
}

fn main() {}
