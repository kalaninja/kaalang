use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Create an unused local wire.")]
    |flag| {
        #[question("Repeat?")]
        let (again, done) = |flag| flag;

        |done| break;

        #[action("Create the unused local wire.")]
        let local = |again| 1;
    };

    return;
}

fn main() {}
