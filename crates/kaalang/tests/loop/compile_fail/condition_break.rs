use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    |&flag| loop {
        #[question("Repeat?")]
        let (_iterate_1, leave_1) = |flag| {
            if flag {
                break;
            }
            false
        };
        |leave_1| break;
    };
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
