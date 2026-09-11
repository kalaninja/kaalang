use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    |&flag| loop {
        #[question("Repeat?")]
        let (iterate_1, leave_1) = |flag| flag;
        |leave_1| break;
        #[action("Escape.")]
        |iterate_1| {
            continue;
        };
    };
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
