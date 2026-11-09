use kaalang::kaa;

#[kaa]
fn invalid(mut count: u8) -> u8 {
    #[kaa::cycle("Count forever.")]
    let found = loop {
        #[kaa::action("Find the count.")]
        let found = |count| count;

        #[kaa::cycle("Count up without end.")]
        loop {
            #[kaa::action("Count one more.")]
            let stepped = |&mut count| *count += 1;

            |stepped| continue;
        };
    };

    |found| return found;
}

fn main() {}
