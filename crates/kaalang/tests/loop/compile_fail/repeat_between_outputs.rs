use kaalang::kaalang;

#[kaalang]
fn invalid(mut values: Vec<i32>) -> i32 {
    #[cycle("Take the next positive value.")]
    let (positive, empty) = {
        #[choice("Is another value waiting?")]
        #[case("Check the value.")]
        #[case("The values ran out.")]
        let (value, empty) = |&mut values| match values.pop() {
            Some(value) => value,
            None => (),
        };

        #[question("Is it positive?")]
        let (keep, skip) = |&value| *value > 0;

        #[action("Keep the positive value.")]
        let positive = |keep, value| value;

        |skip| continue;
    };

    #[question("Is the positive value large?")]
    let (large, small) = |&positive| *positive > 10;

    #[action("Cap the large value.")]
    let result = |large, positive| 10;

    #[action("Keep the small value.")]
    let result = |small, positive| positive;

    #[action("Report nothing found.")]
    let result = |empty| 0;

    |result| return result;
}

fn main() {}
