use kaalang::kaa;

#[kaa]
fn invalid(limit: usize) -> usize {
    #[kaa::action("Start counting at zero.")]
    let mut count = || 0;

    #[kaa::cycle("Count to the limit.")]
    let done = {
        #[kaa::question("Has the limit been reached?")]
        let (done, again) = |&count, &limit| *count >= *limit;

        #[kaa::action("Increment the counter.")]
        |again, &mut count| *count += 1;

        |again| continue;
    };

    |done, count| return count;
}

fn main() {}
