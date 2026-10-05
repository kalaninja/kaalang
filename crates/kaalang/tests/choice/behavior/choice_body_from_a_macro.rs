use kaalang::kaalang;

/// A flow written by another macro, whose choice body arrives wrapped in the
/// invisible group a metavariable substitutes through.
macro_rules! magnitude {
    ($name:ident, $value:ident, $body:expr) => {
        #[kaalang]
        fn $name($value: i32) -> i32 {
            #[choice("Is the value negative?")]
            #[case("Negative.")]
            #[case("Not negative.")]
            let (negative, other) = |$value| $body;

            #[action("Negate the value.")]
            let magnitude = |negative, $value| -$value;

            #[action("Keep the value.")]
            let magnitude = |other, $value| $value;

            |magnitude| return magnitude;
        }
    };
}

magnitude!(
    choice_body_from_a_macro,
    value,
    match value {
        value if value < 0 => (),
        _ => (),
    }
);

#[test]
fn a_choice_body_from_a_macro_selects_its_case() {
    assert_eq!(choice_body_from_a_macro(-3), 3);
    assert_eq!(choice_body_from_a_macro(4), 4);
}
