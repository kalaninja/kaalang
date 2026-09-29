use kaalang::kaalang;

#[kaalang]
fn or(a: bool, b: bool, c: bool) -> bool {
    #[question("Is condition `a` true?")]
    #[yes("YES")]
    #[no("NO")]
    let (true_result, check_b) = |a| a;

    #[question("Is condition `b` true?")]
    #[yes("YES")]
    #[no("NO")]
    let (true_result, check_c) = |check_b, b| b;

    #[question("Is condition `c` true?")]
    #[yes("YES")]
    #[no("NO")]
    let (true_result, false_result) = |check_c, c| c;

    #[action("✅ At least one condition is true; return `true`.")]
    let result = |true_result| true;

    #[action("❌ All three conditions are false; return `false`.")]
    let result = |false_result| false;

    |result| return result;
}

#[test]
fn or_matches_the_boolean_operator() {
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                assert_eq!(or(a, b, c), a || b || c);
            }
        }
    }
}
