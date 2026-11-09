use kaalang::kaa;

#[kaa]
fn branch_local_stage_exit_after_merge(go: bool) -> u8 {
    #[kaa::stage("Finish after a merge.")]
    let finish = |go| {
        #[kaa::question("Take the first branch?")]
        let (first, second) = |go| go;

        #[kaa::action("Build the first value.")]
        let shared = |first| 1u8;

        #[kaa::action("Build the second value.")]
        let shared = |second| 2u8;

        #[kaa::action("Use the merged value.")]
        let used = |shared| shared;

        #[kaa::action("Return through the first branch.")]
        let finish = |first, used| used;

        #[kaa::action("Return through the second branch.")]
        let finish = |second, used| used;
    };

    #[kaa::stage("Return.")]
    |finish| {
        |finish| return finish;
    };
}

fn main() {}
