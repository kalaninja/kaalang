use kaalang::kaalang;

#[kaalang]
fn for_with_an_endless_body(values: &[u32]) -> u32 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Spin on the first value, if there is one.")]
    |values| {
        for value in values {
            #[action("Take the value.")]
            |value, &mut total| *total = *value;

            #[cycle("Spin forever.")]
            loop {
                #[action("Wait.")]
                let waited = || std::hint::spin_loop();

                |waited| continue;
            }
        }
    };

    |total| return total;
}

/// Every route through the body diverges, yet the cycle still completes when
/// its iterator yields nothing.
#[test]
fn a_for_cycle_completes_when_its_body_never_ends_an_iteration() {
    assert_eq!(for_with_an_endless_body(&[]), 0);
}
