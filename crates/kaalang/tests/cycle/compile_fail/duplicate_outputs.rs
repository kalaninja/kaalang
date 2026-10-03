use kaalang::kaalang;

#[kaalang]
fn invalid(value: usize) -> usize {
    #[cycle("Declare one output twice.")]
    let (found, found) = |value| loop {
        #[action("Produce the output.")]
        let found = |value| value;
    };

    |found| return found;
}

fn main() {}
