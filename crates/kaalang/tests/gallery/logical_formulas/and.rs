use kaalang::kaalang;

#[kaalang]
fn and(a: bool, b: bool, c: bool) -> bool {
    #[question("a")]
    #[yes("Yes")]
    #[no("No")]
    |a| -> (check_b, false_result) { a };

    #[question("b")]
    #[yes("Yes")]
    #[no("No")]
    |check_b, b| -> (check_c, false_result) { b };

    #[question("c")]
    #[yes("Yes")]
    #[no("No")]
    |check_c, c| -> (true_result, false_result) { c };

    #[action("Return true.")]
    |true_result| -> result { true };

    #[action("Return false.")]
    |false_result| -> result { false };
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
