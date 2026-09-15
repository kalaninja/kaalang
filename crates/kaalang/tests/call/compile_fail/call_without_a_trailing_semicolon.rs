use kaalang::kaalang;

fn record() {}

#[kaalang]
fn invalid(limit: usize) -> usize {
    #[cycle("Count to the limit.")]
    let total = |limit| {
        #[question("Is the limit reached?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_1, leave_1) = |&limit| *limit > 0;

        |leave_1, limit| break limit;

        #[action("Note the iteration.")]
        |iterate_1| {};

        #[call]
        record()
    };

    |total| return total;
}

fn main() {}
