use kaalang::kaa;

#[kaa]
fn unknown_kaa_attribute(value: i32) -> i32 {
    #[kaa::actoin("Double the value.")]
    let doubled = |value| { value * 2 };
    |doubled| return doubled;
}

fn main() {}
