use kaalang::kaalang;

#[kaalang]
fn for_stepped_range(limit: u32) -> u32 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Add every even number up to the limit.")]
    |limit| {
        for even in (0..=limit).step_by(2) {
            #[action("Add the number.")]
            |even, &mut total| *total += even;
        }
    };

    |total| return total;
}

#[test]
fn a_stepped_range_skips_between_its_items() {
    assert_eq!(for_stepped_range(0), 0);
    assert_eq!(for_stepped_range(5), 6);
    assert_eq!(for_stepped_range(6), 12);
}
