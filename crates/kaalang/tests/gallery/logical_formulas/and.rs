use kaalang::kaalang;

#[kaalang]
fn and(a: bool, b: bool, c: bool) -> bool {
    #[question("a")]
    #[yes("YES")]
    #[no("NO")]
    let (check_b, false_result) = |a| a;

    #[question("b")]
    #[yes("YES")]
    #[no("NO")]
    let (check_c, false_result) = |check_b, b| b;

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
fn and_matches_the_boolean_operator() {
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                assert_eq!(and(a, b, c), a && b && c);
            }
        }
    }
}
