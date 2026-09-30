use kaalang::kaalang;

fn value<T>() -> usize {
    core::mem::size_of::<T>()
}

#[kaalang]
fn uncaptured_generic_parameter_item(value: fn() -> usize) -> (usize, usize) {
    #[action("Call the captured input.")]
    let original = |value| value();

    #[action("Use an uncaptured parameter spelling with generic arguments.")]
    let uncaptured = || value::<u32>();

    |original, uncaptured| return (original, uncaptured);
}

fn main() {}
