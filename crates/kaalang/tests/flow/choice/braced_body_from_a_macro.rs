use kaalang::kaa;

macro_rules! magnitude {
    ($name:ident, $value:ident, $body:expr) => {
        #[kaa]
        fn $name($value: i32) -> i32 {
            #[kaa::choice("Is the value negative?")]
            #[kaa::case("Negative.")]
            #[kaa::case("Not negative.")]
            let (negative, other) = |$value| $body;

            #[kaa::action("Negate the value.")]
            let magnitude = |negative, $value| -$value;

            #[kaa::action("Keep the value.")]
            let magnitude = |other, $value| $value;

            |magnitude| return magnitude;
        }
    };
}

magnitude!(braced_body_from_a_macro, value, {
    match value {
        value if value < 0 => (),
        _ => (),
    }
});

#[test]
fn a_braced_choice_body_from_a_macro_selects_its_case() {
    assert_eq!(braced_body_from_a_macro(-3), 3);
    assert_eq!(braced_body_from_a_macro(4), 4);
}
