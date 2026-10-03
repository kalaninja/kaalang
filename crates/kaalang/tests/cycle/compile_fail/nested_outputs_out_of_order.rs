use kaalang::kaalang;

#[kaalang]
fn invalid(mut count: u32) -> u32 {
    #[cycle("Run the outer cycle.")]
    let result = loop {
        #[cycle("Settle the count.")]
        let (ready, retry) = loop {
            #[question("Is the count positive?")]
            let (retry, ready) = |&count| *count > 0;
        };

        #[action("Count down.")]
        let stepped = |retry, &mut count| *count -= 1;

        |stepped| continue;

        #[action("Keep the count.")]
        let result = |ready, count| count;
    };

    |result| return result;
}

fn main() {}
