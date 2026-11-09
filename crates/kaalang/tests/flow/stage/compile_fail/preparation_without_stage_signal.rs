use kaalang::kaa;

#[kaa]
fn preparation_without_stage_signal() {
    #[kaa::action("Prepare without selecting a stage.")]
    || {};

    #[kaa::stage("Finish.")]
    |finish| {
        return;
    };
}

fn main() {}
