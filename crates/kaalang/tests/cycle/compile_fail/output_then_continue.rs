use kaalang::kaa;

#[kaa]
fn invalid(mut count: usize) -> usize {
    #[kaa::cycle("Produce the output, then repeat anyway.")]
    let total = loop {
        #[kaa::action("Produce the total.")]
        let total = |&count| *count;

        #[kaa::question("Is the count below ten?")]
        let (again, _done) = |&mut count| {
            *count += 1;
            *count < 10
        };

        |again| continue;
    };

    |total| return total;
}

fn main() {}
