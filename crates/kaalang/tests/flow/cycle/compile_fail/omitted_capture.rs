use kaalang::kaa;

#[kaa]
fn invalid(count: usize) {
    #[kaa::cycle("Try an implicit environment capture.")]
    let done = loop {
        #[kaa::question("Repeat?")]
        let (again, done) = |count| count > 0;

        #[kaa::action("Change the counter without capturing it.")]
        |again| {
            count -= 1;
        };

        |again| continue;
    };

    |done| return;
}

fn main() {}
