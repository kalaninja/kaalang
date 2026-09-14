use kaalang::kaalang;

#[kaalang]
fn invalid(count: usize) {
    #[cycle("Try an implicit environment capture.")]
    |mut count| {
        #[question("Repeat?")]
        let (again, done) = |count| count > 0;

        |done| break;

        #[action("Change the counter without capturing it.")]
        |again| {
            count -= 1;
        };
    };

    return;
}

fn main() {}
