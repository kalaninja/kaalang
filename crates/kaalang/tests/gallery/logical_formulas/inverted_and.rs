use kaalang::kaa;

#[kaa]
fn inverted_and(a: bool, b: bool, c: bool) -> bool {
    #[kaa::question("Is condition `a` true?")]
    #[kaa::no("NO")]
    #[kaa::yes("YES")]
    let (true_result, check_b) = |a| a;

    #[kaa::question("Is condition `b` true?")]
    #[kaa::no("NO")]
    #[kaa::yes("YES")]
    let (true_result, check_c) = |check_b, b| b;

    #[kaa::question("Is condition `c` true?")]
    #[kaa::no("NO")]
    #[kaa::yes("YES")]
    let (true_result, false_result) = |check_c, c| c;

    #[kaa::action("✅ At least one condition is false; return `true`.")]
    let result = |true_result| true;

    #[kaa::action("❌ All three conditions are true; return `false`.")]
    let result = |false_result| false;

    |result| return result;
}

#[test]
fn inverted_and_negates_the_boolean_operator() {
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                assert_eq!(inverted_and(a, b, c), !(a && b && c));
            }
        }
    }
}
