use kaalang::kaalang;

#[kaalang]
fn borrowed_stage_header_entry(go: ()) {
    #[stage("Finish.")]
    |&go| {
        return;
    };
}

fn main() {}
