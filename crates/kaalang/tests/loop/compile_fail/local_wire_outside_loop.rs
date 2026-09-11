use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    |&flag| loop {
        #[question("Repeat?")]
        let (iterate_1, leave_1) = |flag| flag;
        |leave_1| break;
        #[action("Create a local wire.")]
        let _local = |iterate_1| 1;
    };
    #[action("Read the local wire.")]
    |_local| {};
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
