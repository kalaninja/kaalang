use kaalang::kaalang;

#[kaalang]
fn invalid(count: usize) {
    #[cycle("Try an implicit environment capture.")]
    let done = loop {
        #[question("Repeat?")]
        let (again, done) = |count| count > 0;

        #[action("Change the counter without capturing it.")]
        |again| {
            count -= 1;
        };

        |again| continue;
    };

    |done| return;
}

fn main() {}
