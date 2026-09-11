use kaalang::kaalang;

#[kaalang]
fn invalid(mut count: usize) {
    |&count| loop {
        #[question("Repeat?")]
        let (iterate_1, leave_1) = |count| count > 0;
        |leave_1| break;
        #[action("Change the counter without capturing it.")]
        |iterate_1| {
            count -= 1;
        };
    };
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
