use kaalang::kaa;

#[kaa]
fn invalid(value: usize) -> usize {
    #[kaa::cycle("Declare an output the body never produces.")]
    let (left, right) = |value| loop {
        #[kaa::action("Produce only the left output.")]
        let left = |value| value;
    };

    #[kaa::action("Use the left output.")]
    let result = |left| left;

    #[kaa::action("Use the right output.")]
    let result = |right| right;

    |result| return result;
}

fn main() {}
