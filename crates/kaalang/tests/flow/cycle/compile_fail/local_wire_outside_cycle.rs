use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) {
    #[kaa::cycle("Create a local wire.")]
    let done = |flag| loop {
        #[kaa::question("Repeat?")]
        let (again, done) = |flag| flag;

        #[kaa::action("Create the local wire.")]
        let _local = |again| 1;
    };

    #[kaa::action("Read the local wire.")]
    |_local| {};

    |done| return;
}

fn main() {}
