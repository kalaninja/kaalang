// Hoisted items deliberately exercise Rust's block-local resolution.
#![allow(clippy::items_after_statements)]

use kaalang::kaa;

fn value() -> u32 {
    9
}

macro_rules! declare_value {
    ($name:ident) => {
        fn $name() -> u32 {
            11
        }
    };
}

#[kaa]
fn parameter_item_paths(value: fn() -> u32) -> (u32, u32, u32, u32) {
    #[kaa::action("Call the captured input.")]
    let captured = |value| value();

    #[kaa::action("Call a qualified module item.")]
    let qualified = || self::value();

    #[kaa::action("Call a hoisted authored local item.")]
    let local = || {
        let result = value();
        fn value() -> u32 {
            7
        }
        result
    };

    #[kaa::action("Call a hoisted item introduced by a macro.")]
    let generated = || {
        let result = value();
        declare_value!(value);
        result
    };

    |captured, qualified, local, generated| return (captured, qualified, local, generated);
}

#[test]
fn qualified_and_explicit_local_items_preserve_rust_resolution() {
    assert_eq!(parameter_item_paths(|| 5), (5, 9, 7, 11));
}
