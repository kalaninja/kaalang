use kaalang::kaalang;

fn twice(value: u32) -> u32 {
    value * 2
}

/// A flow written by another macro. Its callee reaches the parser wrapped in
/// the invisible group a metavariable substitutes through.
macro_rules! applying {
    ($name:ident, $callee:path) => {
        #[kaalang]
        fn $name(value: u32) -> u32 {
            #[call]
            let end = |value| $callee(value);

            |end| return end;
        }
    };
}

applying!(call_a_callee_from_a_macro, twice);

#[test]
fn a_callee_may_arrive_through_a_macro_metavariable() {
    assert_eq!(call_a_callee_from_a_macro(21), 42);
}
