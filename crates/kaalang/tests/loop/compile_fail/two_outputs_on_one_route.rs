use kaalang::kaalang;

#[kaalang]
fn invalid(value: u8) -> u8 {
    #[cycle("Produce both outputs on one route.")]
    let (low, high) = |value| {
        #[action("Produce the low output.")]
        let low = |value| value;

        #[action("Produce the high output too.")]
        let high = |value| value + 1;
    };

    #[action("Keep the low output.")]
    let result = |low| low;

    #[action("Keep the high output.")]
    let result = |high| high;

    |result| return result;
}

fn main() {}
