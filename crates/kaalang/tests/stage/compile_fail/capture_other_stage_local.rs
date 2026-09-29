use kaalang::kaalang;

#[kaalang]
fn capture_other_stage_local(first: ()) -> u8 {
    #[stage("First.")]
    let second = |first| {
        #[action("Make a local.")]
        let hidden = || 1u8;
        #[action("Use the local.")]
        |hidden| ();
        #[action("Select second.")]
        let second = |first| ();
    };
    #[stage("Second.")]
    let finish = |second| {
        #[action("Read the earlier local.")]
        let finish = |hidden| hidden;
    };
    #[stage("Return.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
