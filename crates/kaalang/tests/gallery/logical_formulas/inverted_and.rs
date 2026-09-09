use kaalang::kaalang;

#[kaalang]
fn inverted_and(a: bool, b: bool, c: bool) -> bool {
    #[question("a")]
    #[no("No")]
    #[yes("Yes")]
    |a| -> (true_result, check_b) { a };

    #[question("b")]
    #[no("No")]
    #[yes("Yes")]
    |check_b, b| -> (true_result, check_c) { b };

    #[question("c")]
    #[no("No")]
    #[yes("Yes")]
    |check_c, c| -> (true_result, false_result) { c };

    #[action("Return true.")]
    |true_result| -> result { true };

    #[action("Return false.")]
    |false_result| -> result { false };
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
