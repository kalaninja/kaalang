use kaalang::kaalang;

#[kaalang]
fn invalid(values: &[u32]) -> u32 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Visit every value.")]
    |values| for value in values {
        #[action("Count the visit.")]
        |&mut total| *total += 1;
    };

    |total| return total;
}

fn main() {}
