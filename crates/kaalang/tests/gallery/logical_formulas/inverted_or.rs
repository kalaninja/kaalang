use kaalang::kaalang;

#[kaalang]
fn inverted_or(a: bool, b: bool, c: bool) -> bool {
    #[question("a")]
    #[no("No")]
    #[yes("Yes")]
    let (check_b, false_result) = |a| a;

    #[question("b")]
    #[no("No")]
    #[yes("Yes")]
    let (check_c, false_result) = |check_b, b| b;

    #[question("c")]
    #[no("No")]
    #[yes("Yes")]
    let (true_result, false_result) = |check_c, c| c;

    #[action("Return true.")]
    let result = |true_result| true;

    #[action("Return false.")]
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
