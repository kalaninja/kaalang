use kaalang::kaalang;

/// A cycle cannot return from the flow. An `abort` output hands the decision
/// to the containing sequence, which returns.
#[kaalang]
fn abort_output(mut count: u32, limit: u32) -> Result<u32, u32> {
    #[cycle("Count until done or aborted.")]
    let (done, abort) = {
        #[choice("Where is the count?")]
        #[case("Below the limit.")]
        #[case("At the limit.")]
        #[case("Past the limit.")]
        let (below, done, abort) = |&count, &limit| match count.cmp(limit) {
            std::cmp::Ordering::Less => (),
            std::cmp::Ordering::Equal => (),
            std::cmp::Ordering::Greater => (),
        };

        #[action("Count up.")]
        let stepped = |below, &mut count| *count += 1;

        |stepped| continue;
    };

    #[action("Report the count.")]
    let result = |done, count| Ok(count);

    #[action("Report the abort.")]
    let result = |abort, count| Err(count);

    |result| return result;
}

#[test]
fn the_abort_output_returns_an_error() {
    assert_eq!(abort_output(0, 3), Ok(3));
    assert_eq!(abort_output(3, 3), Ok(3));
    assert_eq!(abort_output(5, 3), Err(5));
}
