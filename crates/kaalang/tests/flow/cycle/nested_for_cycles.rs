use kaalang::kaa;

#[kaa]
fn nested_for_cycles(rows: &[Vec<u32>]) -> u32 {
    #[kaa::action("Start with a total of zero.")]
    let mut total = || 0;

    #[kaa::cycle("Add every row.")]
    |rows| {
        for row in rows {
            #[kaa::cycle("Add every value of the row.")]
            |row| {
                for value in row {
                    #[kaa::action("Add the value.")]
                    |value, &mut total| *total += *value;
                }
            };
        }
    };

    |total| return total;
}

#[test]
fn an_inner_for_cycle_takes_its_own_items() {
    assert_eq!(nested_for_cycles(&[]), 0);
    assert_eq!(nested_for_cycles(&[vec![1, 2], vec![], vec![3]]), 6);
}
