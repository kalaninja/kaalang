use kaalang::kaa;

#[kaa]
fn raw_duplicate_outputs() -> u8 {
    #[kaa::cycle("Declare the same result twice.")]
    let (found, r#found) = loop {
        #[kaa::action("Produce the result.")]
        let found = || 7u8;
    };

    |found| return found;
}

fn main() {}
