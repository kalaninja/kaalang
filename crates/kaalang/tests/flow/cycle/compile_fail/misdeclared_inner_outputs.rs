use kaalang::kaa;

#[kaa]
fn invalid(mut count: u8) -> u8 {
    #[kaa::cycle("Settle the count.")]
    let (large, stopped) = loop {
        #[kaa::question("Is the count large?")]
        let (big, small) = |&count| *count >= 10;

        #[kaa::action("Keep the large count.")]
        let large = |big, &count| *count;

        #[kaa::cycle("Step the small count.")]
        let (up, stop) = |small| loop {
            #[kaa::question("Has it reached three?")]
            let (stop, up) = |&count| *count == 3;
        };

        #[kaa::action("Report the stop.")]
        let stopped = |stop| 3;

        #[kaa::action("Raise the count.")]
        let raised = |up, &mut count| *count += 4;

        |raised| continue;
    };

    #[kaa::action("Use the large count.")]
    let result = |large| large;

    #[kaa::action("Use the stop.")]
    let result = |stopped| stopped * 10;

    |result| return result;
}

fn main() {}
