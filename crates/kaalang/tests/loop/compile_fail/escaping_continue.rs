use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Check the flag.")]
    |flag| {
        #[question("Repeat?")]
        let (again, leave) = |flag| flag;

        |leave| break;

        #[action("Escape from the action.")]
        |again| {
            continue;
        };
    };

    return;
}

fn main() {}
