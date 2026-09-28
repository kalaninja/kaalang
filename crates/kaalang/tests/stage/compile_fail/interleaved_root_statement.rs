use kaalang::kaalang;

#[kaalang]
fn interleaved_root_statement(go: ()) {
    #[stage("Finish.")]
    |go| {
        return;
    };
    #[action("Too late.")]
    || ();
}

fn main() {}
