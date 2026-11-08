use kaalang::kaa;

#[kaa]
fn empty_outputless_stage(start: ()) {
    #[kaa::action("Enter the stage.")]
    let go = |start| start;

    #[kaa::stage("Empty.")]
    |go| {};
}

fn main() {}
