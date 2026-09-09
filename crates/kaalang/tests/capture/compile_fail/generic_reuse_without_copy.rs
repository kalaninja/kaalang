use kaalang::kaalang;

// The generic wire is taken twice, and nothing bounds it by `Copy`.
#[kaalang]
fn invalid<T>(value: T) -> (T, T) {
    #[action("Take the value.")]
    |value| -> first { value };

    #[action("Take the value again.")]
    |value| -> second { value };

    #[action("Pair the two values.")]
    |first, second| -> result { (first, second) };
}

fn main() {}
