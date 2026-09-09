use kaalang::kaalang;

// The generic wire is taken twice, and nothing bounds it by `Copy`.
#[kaalang]
fn invalid<T>(value: T) -> (T, T) {
    #[action("Take the value.")]
    let first = |value| { value };

    #[action("Take the value again.")]
    let second = |value| { value };

    #[action("Pair the two values.")]
    let result = |first, second| { (first, second) };
}

fn main() {}
