use kaalang::kaa;

fn value() -> u32 {
    9
}

#[kaa]
fn uncaptured_parameter_item(value: fn() -> u32) -> u32 {
    #[kaa::action("Read an uncaptured parameter spelling shared with an item.")]
    let result = || value();

    |result| return result;
}

fn main() {}
