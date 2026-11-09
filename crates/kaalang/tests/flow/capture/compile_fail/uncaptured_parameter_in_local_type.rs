use kaalang::kaa;

const fn value() -> usize {
    2
}

#[kaa]
fn uncaptured_parameter_in_local_type(value: fn() -> usize) -> (usize, usize) {
    #[kaa::action("Call the captured input.")]
    let original = |value| value();

    #[kaa::action("Use an uncaptured parameter spelling in a local type.")]
    let uncaptured = || {
        let value: [u8; value()] = [3; 2];
        value.len()
    };

    |original, uncaptured| return (original, uncaptured);
}

fn main() {}
