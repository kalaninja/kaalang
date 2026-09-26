use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Check the flag.")]
    let leave = |flag| {
        #[question("Repeat?")]
        let (again, leave) = |flag| flag;

        #[action("Escape from the action.")]
        |again| {
            continue;
        };
    };

    |leave| return;
}

fn main() {}
