use kaalang::kaalang;

#[kaalang]
fn branch_local_stage_exit_after_merge(go: bool) -> u8 {
    #[stage("Finish after a merge.")]
    let finish = |go| {
        #[question("Take the first branch?")]
        let (first, second) = |go| go;

        #[action("Build the first value.")]
        let shared = |first| 1u8;

        #[action("Build the second value.")]
        let shared = |second| 2u8;

        #[action("Use the merged value.")]
        let used = |shared| shared;

        #[action("Return through the first branch.")]
        let finish = |first, used| used;

        #[action("Return through the second branch.")]
        let finish = |second, used| used;
    };

    #[stage("Return.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
