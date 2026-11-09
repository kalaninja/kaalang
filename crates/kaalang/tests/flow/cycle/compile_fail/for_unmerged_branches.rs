use kaalang::kaa;

#[kaa]
fn invalid(values: Vec<u32>) -> u32 {
    #[kaa::action("Start with a total of zero.")]
    let mut total = || 0;

    #[kaa::cycle("Add the even values.")]
    |values| for value in values {
        #[kaa::question("Is the value even?")]
        let (even, _odd) = |value| value % 2 == 0;

        #[kaa::action("Add the value.")]
        |even, value, &mut total| *total += value;
    };

    |total| return total;
}

fn main() {}
