use kaalang::kaalang;

#[kaalang]
fn preparation_without_stage_signal() {
    #[action("Prepare without selecting a stage.")]
    || {};

    #[stage("Finish.")]
    |finish| {
        return;
    };
}

fn main() {}
