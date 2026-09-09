use kaalang::kaalang;

#[kaalang]
fn or(a: bool, b: bool, c: bool) -> bool {
    #[question("a")]
    #[yes("Yes")]
    #[no("No")]
    |a| -> (true_result, check_b) { a };

    #[question("b")]
    #[yes("Yes")]
    #[no("No")]
    |check_b, b| -> (true_result, check_c) { b };

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
fn or_matches_the_boolean_operator() {
    for a in [false, true] {
        for b in [false, true] {
            for c in [false, true] {
                assert_eq!(or(a, b, c), a || b || c);
            }
        }
    }
}
