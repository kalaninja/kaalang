use kaalang::kaalang;

#[kaalang]
fn invalid(mut count: u8) -> u8 {
    #[cycle("Settle the count.")]
    let (found, other) = loop {
        #[question("Is the count even?")]
        let (even, odd) = |&count| *count % 2 == 0;

        #[question("Is it still even?")]
        let (again, even_found) = |even, &count| *count % 2 == 0;

        #[question("Is it a multiple of three?")]
        let (third, rest) = |odd, &count| *count % 3 == 0;

        #[action("Step the count.")]
        let stepped = |again, &mut count| *count = count.wrapping_add(1);

        #[action("Find it as even.")]
        let found = |even_found| 6;

        #[action("Find it as a multiple of three.")]
        let found = |third| 9;

        #[action("Report the rest.")]
        let other = |rest| 6;

        |stepped| continue;
    };

    #[action("Use the found count.")]
    let result = |found| 5;

    #[action("Use the rest.")]
    let result = |other| 4;

    |result| return result;
}

fn main() {}
