use kaalang::kaalang;

#[kaalang]
fn invalid(limit: usize) -> usize {
    #[action("Start counting at zero.")]
    let mut count = || 0;

    #[cycle("Count to the limit.")]
    let done = {
        #[question("Has the limit been reached?")]
        let (done, again) = |&count, &limit| *count >= *limit;

        #[action("Increment the counter.")]
        |again, &mut count| *count += 1;

        |again| continue;
    };

    |done, count| return count;
}

fn main() {}
