use kaalang::kaa;

#[kaa]
fn and(a: bool, b: bool, c: bool) -> bool {
    #[kaa::question("Is condition `a` true?")]
    #[kaa::yes("YES")]
    #[kaa::no("NO")]
    let (check_b, false_result) = |a| a;

    #[kaa::question("Is condition `b` true?")]
    #[kaa::yes("YES")]
    #[kaa::no("NO")]
    let (check_c, false_result) = |check_b, b| b;

    #[kaa::question("Is condition `c` true?")]
    #[kaa::yes("YES")]
    #[kaa::no("NO")]
    let (true_result, false_result) = |check_c, c| c;

    #[kaa::action("✅ All three conditions are true; return `true`.")]
    let result = |true_result| true;

    #[kaa::action("❌ At least one condition is false; return `false`.")]
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
