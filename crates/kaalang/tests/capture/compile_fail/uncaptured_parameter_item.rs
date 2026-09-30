use kaalang::kaalang;

fn value() -> u32 {
    9
}

#[kaalang]
fn uncaptured_parameter_item(value: fn() -> u32) -> u32 {
    #[action("Read an uncaptured parameter spelling shared with an item.")]
    let result = || value();

    |result| return result;
}

fn main() {}
