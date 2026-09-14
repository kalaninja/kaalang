use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool, value: usize) -> usize {
    #[cycle("Choose a result.")]
    let result = |flag, value| {
        #[question("Use the first route?")]
        let (first, second) = |flag| flag;

        |first, value| break value;
        |second, value| break value;
    };

    |result| return result;
}

fn main() {}
