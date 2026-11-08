use kaalang::kaa;

#[kaa]
fn invalid(mut values: Vec<i32>) -> i32 {
    #[kaa::cycle("Take the next positive value.")]
    let (positive, empty) = loop {
        #[kaa::choice("Is another value waiting?")]
        #[kaa::case("Check the value.")]
        #[kaa::case("The values ran out.")]
        let (value, empty) = |&mut values| match values.pop() {
            Some(value) => value,
            None => (),
        };

        #[kaa::question("Is it positive?")]
        let (keep, skip) = |&value| *value > 0;

        #[kaa::action("Keep the positive value.")]
        let positive = |keep, value| value;

        |skip| continue;
    };

    #[kaa::question("Is the positive value large?")]
    let (large, small) = |&positive| *positive > 10;

    #[kaa::action("Cap the large value.")]
    let result = |large, positive| 10;

    #[kaa::action("Keep the small value.")]
    let result = |small, positive| positive;

    #[kaa::action("Report nothing found.")]
    let result = |empty| 0;

    |result| return result;
}

fn main() {}
