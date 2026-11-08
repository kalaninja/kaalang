use kaalang::kaa;

macro_rules! repeat {
    ($name:ident, $text:ident, $iteration:expr) => {
        #[kaa]
        fn $name() -> String {
            #[kaa::action("Start with an empty text.")]
            let mut $text = || String::new();

            #[kaa::cycle("Knock three times.")]
            $iteration;

            |$text| return $text;
        }
    };
}

repeat!(
    for_cycle_from_a_macro,
    text,
    for _ in 0..3 {
        #[kaa::action("Knock once.")]
        |&mut text| text.push_str("knock ");
    }
);

#[test]
fn a_bare_for_cycle_from_a_macro_runs() {
    assert_eq!(for_cycle_from_a_macro(), "knock knock knock ");
}
