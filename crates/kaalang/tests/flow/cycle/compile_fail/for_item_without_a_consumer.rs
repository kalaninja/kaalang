use kaalang::kaa;

#[kaa]
fn invalid(values: &[u32]) -> u32 {
    #[kaa::action("Start with a total of zero.")]
    let mut total = || 0;

    #[kaa::cycle("Visit every value.")]
    |values| for value in values {
        #[kaa::action("Count the visit.")]
        |&mut total| *total += 1;
    };

    |total| return total;
}

fn main() {}
