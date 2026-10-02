use kaalang::kaalang;

#[kaalang]
fn raw_duplicate_outputs() -> u8 {
    #[cycle("Declare the same result twice.")]
    let (found, r#found) = {
        #[action("Produce the result.")]
        let found = || 7u8;
    };

    |found| return found;
}

fn main() {}
