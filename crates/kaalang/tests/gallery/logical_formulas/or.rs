use kaalang::kaalang;

#[kaalang]
fn or(a: bool, b: bool, c: bool) -> bool {
    #[question("a")]
    #[yes("Yes")]
    #[no("No")]
    let (true_result, check_b) = |a| a;

    #[question("b")]
    #[yes("Yes")]
    #[no("No")]
    let (true_result, check_c) = |check_b, b| b;

    #[question("c")]
    #[yes("Yes")]
    #[no("No")]
    let (true_result, false_result) = |check_c, c| c;

    #[action("Return true.")]
    let end = |true_result| true;

    #[action("Return false.")]
    let end = |false_result| false;
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
