use kaalang::kaa;

#[kaa]
fn interleaved_root_statement(go: ()) {
    #[kaa::stage("Finish.")]
    |go| {
        return;
    };
    #[kaa::action("Too late.")]
    || ();
}

fn main() {}
