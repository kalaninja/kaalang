use kaalang::kaalang;

#[kaalang]
fn inverted_and(a: bool, b: bool, c: bool) -> bool {
    #[question("a")]
    #[no("No")]
    #[yes("Yes")]
    let (true_result, check_b) = |a| a;

    #[question("b")]
    #[no("No")]
    #[yes("Yes")]
    let (true_result, check_c) = |check_b, b| b;

    #[question("c")]
    #[no("No")]
    #[yes("Yes")]
    let (true_result, false_result) = |check_c, c| c;

    #[action("Return true.")]
    let end = |true_result| true;

    #[action("Return false.")]
    let end = |false_result| false;
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
