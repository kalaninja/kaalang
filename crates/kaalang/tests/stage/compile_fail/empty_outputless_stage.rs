use kaalang::kaalang;

#[kaalang]
fn empty_outputless_stage(start: ()) {
    #[action("Enter the stage.")]
    let go = |start| start;

    #[stage("Empty.")]
    |go| {};
}

fn main() {}
