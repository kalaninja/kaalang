use kaalang::kaalang;

#[kaalang]
fn inverted_and(a: bool, b: bool, c: bool) -> bool {
    #[question("a")]
    #[no("NO")]
    #[yes("YES")]
    let (true_result, check_b) = |a| a;

    #[question("b")]
    #[no("NO")]
    #[yes("YES")]
    let (true_result, check_c) = |check_b, b| b;

    #[question("c")]
    #[no("NO")]
    #[yes("YES")]
    let (true_result, false_result) = |check_c, c| c;

    #[action("✅ True.")]
    let result = |true_result| true;

    #[action("❌ False.")]
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
