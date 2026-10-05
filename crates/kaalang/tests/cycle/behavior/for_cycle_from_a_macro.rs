use kaalang::kaalang;

macro_rules! repeat {
    ($name:ident, $text:ident, $iteration:expr) => {
        #[kaalang]
        fn $name() -> String {
            #[action("Start with an empty text.")]
            let mut $text = || String::new();

            #[cycle("Knock three times.")]
            $iteration;

            |$text| return $text;
        }
    };
}

repeat!(
    for_cycle_from_a_macro,
    text,
    for _ in 0..3 {
        #[action("Knock once.")]
        |&mut text| text.push_str("knock ");
    }
);

#[test]
fn a_bare_for_cycle_from_a_macro_runs() {
    assert_eq!(for_cycle_from_a_macro(), "knock knock knock ");
}
