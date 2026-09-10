use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[question("Repeat?")]
    while (|flag| flag) {
        #[action("Create a local wire.")]
        let _local = || 1;
    }
    #[action("Read the local wire.")]
    |_local| {};
    #[action("Finish.")]
    let result = || {};
}

fn main() {}
