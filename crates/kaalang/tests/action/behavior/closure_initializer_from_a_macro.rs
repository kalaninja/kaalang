use kaalang::kaalang;

/// A flow written by another macro, whose action initializer arrives wrapped in
/// the invisible group a metavariable substitutes through.
macro_rules! doubling {
    ($name:ident, $initializer:expr) => {
        #[kaalang]
        fn $name(value: u32) -> u32 {
            #[action("Double the value.")]
            let doubled = $initializer;

            |doubled| return doubled;
        }
    };
}

doubling!(closure_initializer_from_a_macro, |value| value * 2);

#[test]
fn a_closure_initializer_from_a_macro_keeps_its_capture_list() {
    assert_eq!(closure_initializer_from_a_macro(21), 42);
}
