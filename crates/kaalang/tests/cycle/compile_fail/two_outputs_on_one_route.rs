use kaalang::kaa;

#[kaa]
fn invalid(value: u8) -> u8 {
    #[kaa::cycle("Produce both outputs on one route.")]
    let (low, high) = |value| loop {
        #[kaa::action("Produce the low output.")]
        let low = |value| value;

        #[kaa::action("Produce the high output too.")]
        let high = |value| value + 1;
    };

    #[kaa::action("Keep the low output.")]
    let result = |low| low;

    #[kaa::action("Keep the high output.")]
    let result = |high| high;

    |result| return result;
}

fn main() {}
