use kaalang::kaa;

#[kaa]
fn invalid(value: usize) -> usize {
    #[kaa::cycle("Declare one output twice.")]
    let (found, found) = |value| loop {
        #[kaa::action("Produce the output.")]
        let found = |value| value;
    };

    |found| return found;
}

fn main() {}
