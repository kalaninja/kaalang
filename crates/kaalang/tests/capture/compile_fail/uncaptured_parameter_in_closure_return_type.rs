use kaalang::kaalang;

const fn value() -> usize {
    3
}

#[kaalang]
fn invalid(value: usize) -> usize {
    #[action("Consume the parameter explicitly.")]
    |value| { assert_eq!(value, 100); };

    #[action("Miss a capture in a closure result type.")]
    let result = || {
        let make = || -> [u8; value()] { [1, 2, 3] };
        make().len()
    };

    |result| return result;
}

fn main() {}
