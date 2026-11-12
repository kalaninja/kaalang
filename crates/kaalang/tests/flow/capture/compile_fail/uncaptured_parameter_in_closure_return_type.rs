use kaalang::kaa;

const fn value() -> usize {
    3
}

#[kaa]
fn invalid(value: usize) -> usize {
    #[kaa::action("Consume the parameter explicitly.")]
    |value| { assert_eq!(value, 100); };

    #[kaa::action("Miss a capture in a closure result type.")]
    let result = || {
        let make = || -> [u8; value()] { [1, 2, 3] };
        make().len()
    };

    |result| return result;
}

fn main() {}
