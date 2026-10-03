use kaalang::kaalang;

#[kaalang]
fn invalid(mut count: u8) -> u8 {
    #[cycle("Count forever.")]
    let found = loop {
        #[action("Find the count.")]
        let found = |count| count;

        #[cycle("Count up without end.")]
        loop {
            #[action("Count one more.")]
            let stepped = |&mut count| *count += 1;

            |stepped| continue;
        };
    };

    |found| return found;
}

fn main() {}
