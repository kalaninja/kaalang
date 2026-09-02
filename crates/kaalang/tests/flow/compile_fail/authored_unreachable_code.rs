#![deny(unreachable_code)]

use kaalang::kaalang;

#[kaalang]
fn written(value: i32) -> i32 {
    #[action("Return early, then compute a value that never runs.")]
    |value| -> doubled {
        return value * 2;
        let quadrupled = value * 4;
        quadrupled
    };

    #[action("Produce the result.")]
    |doubled| -> result { doubled };

    #[end]
    |result| {};
}

fn main() {
    let _ = written(1);
}
