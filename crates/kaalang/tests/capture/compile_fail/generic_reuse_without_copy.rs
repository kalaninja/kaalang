use kaalang::kaa;

// The generic wire is taken twice, and nothing bounds it by `Copy`.
#[kaa]
fn invalid<T>(value: T) -> (T, T) {
    #[kaa::action("Take the value.")]
    let first = |value| { value };

    #[kaa::action("Take the value again.")]
    let second = |value| { value };

    #[kaa::action("Pair the two values.")]
    let end = |first, second| { (first, second) };

    |end| return end;
}

fn main() {}
