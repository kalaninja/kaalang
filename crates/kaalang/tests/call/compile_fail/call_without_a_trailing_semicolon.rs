use kaalang::kaalang;

fn record() {}

#[kaalang]
fn invalid(limit: usize) -> usize {
    #[cycle("Count to the limit.")]
    let leave_1 = |limit| loop {
        #[question("Is the limit reached?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_1, leave_1) = |&limit| *limit > 0;

        #[action("Note the iteration.")]
        |iterate_1| {};

        #[call]
        record()
    };

    |leave_1, limit| return limit;
}

fn main() {}
