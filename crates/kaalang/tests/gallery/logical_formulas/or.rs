use kaalang::kaalang;

#[kaalang]
fn or(a: bool, b: bool, c: bool) -> bool {
    #[question("a")]
    #[yes("YES")]
    #[no("NO")]
    let (true_result, check_b) = |a| a;

    #[question("b")]
    #[yes("YES")]
    #[no("NO")]
    let (true_result, check_c) = |check_b, b| b;

    #[question("c")]
    #[yes("YES")]
    #[no("NO")]
    let (true_result, false_result) = |check_c, c| c;

    #[action("✅ True.")]
    let result = |true_result| true;

    #[action("❌ False.")]
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
