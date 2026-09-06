use kaalang::kaalang;

#[kaalang]
fn a_branch_captures_a_merged_value(amount: u8, verbose: bool) -> u8 {
    #[question("Is the amount positive?")]
    |amount| -> (positive, negative) { amount > 0 };

    #[action("Take the positive amount.")]
    |positive| -> (counted, seen) { (11u8, ()) };

    #[action("Take the negative amount.")]
    |negative| -> (counted, seen) { (10u8, ()) };

    #[question("Should the run report anything?")]
    |verbose| -> (report, quiet) { verbose };

    #[action("Report the counted amount.")]
    |report, counted| -> result { counted };

    #[action("Report nothing.")]
    |quiet, seen| -> result { 0u8 };
}

#[test]
fn a_merged_wire_is_ordinary_data_that_one_branch_may_capture() {
    for (amount, verbose, expected) in [(1, true, 11), (0, true, 10), (1, false, 0), (0, false, 0)]
    {
        assert_eq!(a_branch_captures_a_merged_value(amount, verbose), expected);
    }
}
