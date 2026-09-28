use kaalang::kaalang;

#[kaalang]
fn inverted_or(a: bool, b: bool, c: bool) -> bool {
    #[question("Is condition `a` true?")]
    #[no("NO")]
    #[yes("YES")]
    let (check_b, false_result) = |a| a;

    #[question("Is condition `b` true?")]
    #[no("NO")]
    #[yes("YES")]
    let (check_c, false_result) = |check_b, b| b;

    #[question("Is condition `c` true?")]
    #[no("NO")]
    #[yes("YES")]
    let (true_result, false_result) = |check_c, c| c;

    #[action("✅ All three conditions are false; return `true`.")]
    let result = |true_result| true;

    #[action("❌ At least one condition is true; return `false`.")]
    let result = |false_result| false;

    |result| return result;
}

#[test]
fn inverted_or_negates_the_boolean_operator() {
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                assert_eq!(inverted_or(a, b, c), !(a || b || c));
            }
        }
    }
}
