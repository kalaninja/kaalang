use kaalang::kaa;

#[kaa]
fn unbraced_stage_body(go: ()) {
    #[kaa::stage("Finish.")]
    |go| return;
}

fn main() {}
