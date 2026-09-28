use kaalang::kaalang;

#[kaalang]
fn sum_to_limit(limit: u8) -> u32 {
    #[action("Start at zero.")]
    let count = || (0u8, 0u32);

    #[stage("Accumulate up to the limit.")]
    let (count, finish) = |count| {
        #[action("Advance the count and total.")]
        let next = |mut count, limit| {
            if count.0 < limit {
                count.0 += 1;
                count.1 += u32::from(count.0);
            }
            count
        };

        #[choice("Are there more numbers?")]
        #[case("Continue counting.")]
        #[case("Stop counting.")]
        let (count, finish) = |next, limit| match next.0 < limit {
            true => next,
            false => next.1,
        };
    };

    #[stage("Return the total.")]
    |finish| {
        |finish| return finish;
    };
}

#[test]
fn a_mutable_entry_copy_accumulates_across_stage_visits() {
    assert_eq!(sum_to_limit(0), 0);
    assert_eq!(sum_to_limit(4), 10);
    assert_eq!(sum_to_limit(10), 55);
}
