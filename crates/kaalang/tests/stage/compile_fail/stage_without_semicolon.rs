use kaalang::kaalang;

#[kaalang]
fn stage_without_semicolon(go: ()) {
    #[stage("Finish.")]
    |go| {
        return;
    }
}

fn main() {}
