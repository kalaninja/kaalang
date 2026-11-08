use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) {
    #[kaa::cycle("Check the flag.")]
    let leave = |flag| loop {
        #[kaa::question("Repeat?")]
        let (again, leave) = |flag| flag;

        #[kaa::action("Escape from the action.")]
        |again| {
            continue;
        };
    };

    |leave| return;
}

fn main() {}
