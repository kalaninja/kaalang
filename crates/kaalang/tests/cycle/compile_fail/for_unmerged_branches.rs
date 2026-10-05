use kaalang::kaalang;

#[kaalang]
fn invalid(values: Vec<u32>) -> u32 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Add the even values.")]
    |values| for value in values {
        #[question("Is the value even?")]
        let (even, _odd) = |value| value % 2 == 0;

        #[action("Add the value.")]
        |even, value, &mut total| *total += value;
    };

    |total| return total;
}

fn main() {}
