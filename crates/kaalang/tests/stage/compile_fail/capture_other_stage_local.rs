use kaalang::kaa;

#[kaa]
fn capture_other_stage_local(first: ()) -> u8 {
    #[kaa::stage("First.")]
    let second = |first| {
        #[kaa::action("Make a local.")]
        let hidden = || 1u8;
        #[kaa::action("Use the local.")]
        |hidden| ();
        #[kaa::action("Select second.")]
        let second = |first| ();
    };
    #[kaa::stage("Second.")]
    let finish = |second| {
        #[kaa::action("Read the earlier local.")]
        let finish = |hidden| hidden;
    };
    #[kaa::stage("Return.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
