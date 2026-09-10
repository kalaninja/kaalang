use kaalang::kaalang;

#[kaalang]
fn and(a: bool, b: bool, c: bool) -> bool {
    #[question("a")]
    #[yes("Yes")]
    #[no("No")]
    let (check_b, false_result) = |a| a;

    #[question("b")]
    #[yes("Yes")]
    #[no("No")]
    let (check_c, false_result) = |check_b, b| b;

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
fn and_matches_the_boolean_operator() {
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                assert_eq!(and(a, b, c), a && b && c);
            }
        }
    }
}
