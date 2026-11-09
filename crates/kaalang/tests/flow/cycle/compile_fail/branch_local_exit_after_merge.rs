use kaalang::kaa;

#[kaa]
fn branch_local_exit_after_merge(go: bool) -> u8 {
    #[kaa::cycle("Complete after a merge.")]
    let finish = |go| loop {
        #[kaa::question("Take the first branch?")]
        let (first, second) = |go| go;

        #[kaa::action("Build the first value.")]
        let shared = |first| 1u8;

        #[kaa::action("Build the second value.")]
        let shared = |second| 2u8;

        #[kaa::action("Use the merged value.")]
        let used = |shared| shared;

        #[kaa::action("Exit through the first branch.")]
        let finish = |first, used| used;

        #[kaa::action("Exit through the second branch.")]
        let finish = |second, used| used;
    };

    |finish| return finish;
}

fn main() {}
