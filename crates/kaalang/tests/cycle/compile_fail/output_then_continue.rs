use kaalang::kaalang;

#[kaalang]
fn invalid(mut count: usize) -> usize {
    #[cycle("Produce the output, then repeat anyway.")]
    let total = {
        #[action("Produce the total.")]
        let total = |&count| *count;

        #[question("Is the count below ten?")]
        let (again, _done) = |&mut count| {
            *count += 1;
            *count < 10
        };

        |again| continue;
    };

    |total| return total;
}

fn main() {}
