use kaalang::kaa;

#[kaa]
fn identical_stage_descriptions(go: ()) -> u8 {
    #[kaa::stage("Same description.")]
    let finish = |go| {
        #[kaa::action("Select the next stage.")]
        let finish = |go| 7u8;
    };

    #[kaa::stage(r"Same description.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
