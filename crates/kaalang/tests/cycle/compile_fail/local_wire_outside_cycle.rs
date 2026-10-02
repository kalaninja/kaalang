use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Create a local wire.")]
    let done = |flag| {
        #[question("Repeat?")]
        let (again, done) = |flag| flag;

        #[action("Create the local wire.")]
        let _local = |again| 1;
    };

    #[action("Read the local wire.")]
    |_local| {};

    |done| return;
}

fn main() {}
