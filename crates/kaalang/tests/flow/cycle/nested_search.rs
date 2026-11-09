use kaalang::kaa;

#[kaa]
fn nested_search(rows: &[&[i32]], target: i32) -> Option<(usize, usize)> {
    #[kaa::action("Start at the first row.")]
    let mut row = || 0;

    #[kaa::cycle("Search each row.")]
    let outcome = loop {
        #[kaa::question("Are there more rows?")]
        #[kaa::no("All rows have been searched.")]
        #[kaa::yes("Search this row.")]
        let (leave_1, iterate_1) = |&row, &rows| *row < rows.len();

        #[kaa::action("The target is absent.")]
        let outcome = |leave_1| None;

        #[kaa::action("Start at the first column.")]
        let mut column = |iterate_1| 0;

        #[kaa::cycle("Search the current row.")]
        let found = |column| loop {
            #[kaa::question("Are there more columns?")]
            #[kaa::yes("YES")]
            #[kaa::no("NO")]
            let (iterate_2, leave_2) = |&column, &row, &rows| *column < (*rows)[*row].len();

            #[kaa::action("No match exists in this row.")]
            let found = |leave_2| None;

            #[kaa::question("Does the value differ from the target?")]
            let (different, equal) =
                |iterate_2, &rows, &row, &column, &target| (*rows)[*row][*column] != *target;

            #[kaa::action("Advance to the next column.")]
            |different, &mut column| *column += 1;

            #[kaa::action("Produce the matching position.")]
            let found = |equal, &row, &column| Some((*row, *column));

            |different| continue;
        };

        #[kaa::question("Was the target found in this row?")]
        let (done, again) = |&found| found.is_some();

        #[kaa::action("Keep the matching position.")]
        let outcome = |done, found| found;

        #[kaa::action("Advance to the next row.")]
        |again, &mut row| *row += 1;

        |again| continue;
    };

    |outcome| return outcome;
}

#[test]
fn an_inner_cycle_result_can_finish_the_entire_flow() {
    let rows = [&[][..], &[1, 2][..], &[3][..]];
    assert_eq!(nested_search(&[], 1), None);
    assert_eq!(nested_search(&rows, 3), Some((2, 0)));
    assert_eq!(nested_search(&rows, 2), Some((1, 1)));
    assert_eq!(nested_search(&rows, 4), None);
}
