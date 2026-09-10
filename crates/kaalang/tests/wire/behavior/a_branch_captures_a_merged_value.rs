use kaalang::kaalang;

#[kaalang]
fn a_branch_captures_a_merged_value(amount: u8, verbose: bool) -> u8 {
    #[question("Is the amount positive?")]
    let (positive, negative) = |amount| amount > 0;

    #[action("Take the positive amount.")]
    let (counted, seen) = |positive| (11u8, ());

    #[action("Take the negative amount.")]
    let (counted, seen) = |negative| (10u8, ());

    #[question("Should the run report anything?")]
    let (report, quiet) = |verbose| verbose;

    #[action("Report the counted amount.")]
    let end = |report, counted| counted;

    #[action("Report nothing.")]
    let end = |quiet, seen| 0u8;
}

#[test]
fn a_merged_wire_is_ordinary_data_that_one_branch_may_capture() {
    for (amount, verbose, expected) in [(1, true, 11), (0, true, 10), (1, false, 0), (0, false, 0)]
    {
        assert_eq!(a_branch_captures_a_merged_value(amount, verbose), expected);
    }
}
