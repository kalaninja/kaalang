use kaalang::kaalang;

#[kaalang]
fn identical_stage_descriptions(go: ()) -> u8 {
    #[stage("Same description.")]
    let finish = |go| {
        #[action("Select the next stage.")]
        let finish = |go| 7u8;
    };

    #[stage(r"Same description.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
