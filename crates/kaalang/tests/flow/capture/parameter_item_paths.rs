// Hoisted items deliberately exercise Rust's block-local resolution.
#![allow(clippy::items_after_statements)]
#![deny(unused_braces)]

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

macro_rules! invoke {
    ($value:ident) => {
        $value()
    };
}

#[kaa]
fn parameter_item_paths(value: fn() -> u32) -> (u32, u32, u32, u32) {
    #[kaa::action("Call the captured input.")]
    let captured = |value| {
        invoke! {value}
    };

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

#[test]
fn hoisted_items_shadow_a_captured_callback() {
    #[kaa]
    fn dispatch(value: fn() -> u32) -> (u32, u32) {
        #[kaa::action("Call the local function.")]
        let local = |value| {
            fn value() -> u32 {
                7
            }
            value()
        };

        #[kaa::question("Does the local function select this branch?")]
        let (yes, no) = |value| {
            declare_value!(value);
            value() == 11
        };
        #[kaa::action("Use the local result.")]
        let selected = |yes| 11;
        #[kaa::action("Use the other result.")]
        let selected = |no| 0;

        |local, selected| return (local, selected);
    }

    assert_eq!(dispatch(|| 5), (7, 11));
}
