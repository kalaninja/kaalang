use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Create a local wire.")]
    |flag| {
        #[question("Repeat?")]
        let (again, done) = |flag| flag;

        |done| break;

        #[action("Create the local wire.")]
        let _local = |again| 1;
    };

    #[action("Read the local wire.")]
    |_local| {};

    return;
}

fn main() {}
