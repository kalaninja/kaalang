use kaalang::kaalang;

fn scale(value: &u32) -> u32 {
    *value * 100
}

#[kaalang]
fn invalid(values: Vec<u32>, scale: fn(&u32) -> u32) -> u32 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Add every scaled value.")]
    |&values| for value in values.iter().map(scale) {
        #[action("Add the value.")]
        |value, &mut total| *total += value;
    };

    |total, scale| return total;
}

fn main() {}
