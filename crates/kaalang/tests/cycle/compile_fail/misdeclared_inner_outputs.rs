use kaalang::kaalang;

#[kaalang]
fn invalid(mut count: u8) -> u8 {
    #[cycle("Settle the count.")]
    let (large, stopped) = loop {
        #[question("Is the count large?")]
        let (big, small) = |&count| *count >= 10;

        #[action("Keep the large count.")]
        let large = |big, &count| *count;

        #[cycle("Step the small count.")]
        let (up, stop) = |small| loop {
            #[question("Has it reached three?")]
            let (stop, up) = |&count| *count == 3;
        };

        #[action("Report the stop.")]
        let stopped = |stop| 3;

        #[action("Raise the count.")]
        let raised = |up, &mut count| *count += 4;

        |raised| continue;
    };

    #[action("Use the large count.")]
    let result = |large| large;

    #[action("Use the stop.")]
    let result = |stopped| stopped * 10;

    |result| return result;
}

fn main() {}
