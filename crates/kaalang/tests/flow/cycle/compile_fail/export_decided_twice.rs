use kaalang::kaa;

#[kaa]
fn invalid(mut count: u8) -> u8 {
    #[kaa::cycle("Settle the count.")]
    let (found, other) = loop {
        #[kaa::question("Is the count even?")]
        let (even, odd) = |&count| *count % 2 == 0;

        #[kaa::question("Is it still even?")]
        let (again, even_found) = |even, &count| *count % 2 == 0;

        #[kaa::question("Is it a multiple of three?")]
        let (third, rest) = |odd, &count| *count % 3 == 0;

        #[kaa::action("Step the count.")]
        let stepped = |again, &mut count| *count = count.wrapping_add(1);

        #[kaa::action("Find it as even.")]
        let found = |even_found| 6;

        #[kaa::action("Find it as a multiple of three.")]
        let found = |third| 9;

        #[kaa::action("Report the rest.")]
        let other = |rest| 6;

        |stepped| continue;
    };

    #[kaa::action("Use the found count.")]
    let result = |found| 5;

    #[kaa::action("Use the rest.")]
    let result = |other| 4;

    |result| return result;
}

fn main() {}
