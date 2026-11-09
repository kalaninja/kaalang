use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) {
    #[kaa::cycle("Create an unused local wire.")]
    let done = |flag| loop {
        #[kaa::question("Repeat?")]
        let (again, done) = |flag| flag;

        #[kaa::action("Create the unused local wire.")]
        let local = |again| 1;

        |again| continue;
    };

    |done| return;
}

fn main() {}
