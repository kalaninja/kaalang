use kaalang::kaalang;

#[kaalang]
fn unbraced_stage_body(go: ()) {
    #[stage("Finish.")]
    |go| return;
}

fn main() {}
