use kaalang::kaalang;

#[kaalang]
fn invalid(value: usize) -> usize {
    #[cycle("Declare an output the body never produces.")]
    let (left, right) = |value| loop {
        #[action("Produce only the left output.")]
        let left = |value| value;
    };

    #[action("Use the left output.")]
    let result = |left| left;

    #[action("Use the right output.")]
    let result = |right| right;

    |result| return result;
}

fn main() {}
