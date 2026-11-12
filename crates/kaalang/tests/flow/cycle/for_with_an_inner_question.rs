use kaalang::kaa;

#[kaa]
fn for_with_an_inner_question(values: &[u32], limit: u32) -> u32 {
    #[kaa::action("Start with a total of zero.")]
    let mut total = || 0;

    #[kaa::cycle("Add values while the total is below the limit.")]
    |values| {
        for value in values {
            #[kaa::question("Is the total below the limit?")]
            let (below, _full) = |&total, limit| *total < limit;

            #[kaa::action("Add the value.")]
            let _full = |below, value, &mut total| *total += *value;
        }
    };

    |total| return total;
}

/// The question reads outer state rather than the item, and only one branch
/// uses the item.
#[test]
fn a_question_in_a_for_body_need_not_read_the_item() {
    assert_eq!(for_with_an_inner_question(&[], 8), 0);
    assert_eq!(for_with_an_inner_question(&[5, 5, 5], 8), 10);
}
