use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Create an unused local wire.")]
    let done = |flag| {
        #[question("Repeat?")]
        let (again, done) = |flag| flag;

        #[action("Create the unused local wire.")]
        let local = |again| 1;

        |again| continue;
    };

    |done| return;
}

fn main() {}
