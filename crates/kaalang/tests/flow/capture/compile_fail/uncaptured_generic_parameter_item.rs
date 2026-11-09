use kaalang::kaa;

fn value<T>() -> usize {
    core::mem::size_of::<T>()
}

#[kaa]
fn uncaptured_generic_parameter_item(value: fn() -> usize) -> (usize, usize) {
    #[kaa::action("Call the captured input.")]
    let original = |value| value();

    #[kaa::action("Use an uncaptured parameter spelling with generic arguments.")]
    let uncaptured = || value::<u32>();

    |original, uncaptured| return (original, uncaptured);
}

fn main() {}
