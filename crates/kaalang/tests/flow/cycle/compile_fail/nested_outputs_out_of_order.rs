use kaalang::kaa;

#[kaa]
fn invalid(mut count: u32) -> u32 {
    #[kaa::cycle("Run the outer cycle.")]
    let result = loop {
        #[kaa::cycle("Settle the count.")]
        let (ready, retry) = loop {
            #[kaa::question("Is the count positive?")]
            let (retry, ready) = |&count| *count > 0;
        };

        #[kaa::action("Count down.")]
        let stepped = |retry, &mut count| *count -= 1;

        |stepped| continue;

        #[kaa::action("Keep the count.")]
        let result = |ready, count| count;
    };

    |result| return result;
}

fn main() {}
