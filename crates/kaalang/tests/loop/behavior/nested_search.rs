use kaalang::kaalang;

#[kaalang]
fn nested_search(rows: &[&[i32]], target: i32) -> Option<(usize, usize)> {
    #[action("Start at the first row.")]
    let mut row = || 0;

    #[cycle("Search each row.")]
    let result = |mut row, rows, target| {
        #[question("Are there more rows?")]
        #[no("All rows have been searched.")]
        #[yes("Search this row.")]
        let (leave_1, iterate_1) = |&row, &rows| *row < rows.len();

        #[action("The target is absent.")]
        let absent = |leave_1| None;

        |absent| break absent;

        #[action("Start at the first column.")]
        let mut column = |iterate_1| 0;

        #[cycle("Search the current row.")]
        let found = |mut column, &row, rows, target| {
            #[question("Are there more columns?")]
            #[yes("YES")]
            #[no("NO")]
            let (iterate_2, leave_2) = |&column, &row, &rows| *column < (*rows)[**row].len();

            #[action("No match exists in this row.")]
            let absent = |leave_2| None;

            |absent| break absent;

            #[question("Does the value differ from the target?")]
            let (different, equal) =
                |iterate_2, &rows, &row, &column, &target| (*rows)[**row][*column] != *target;

            #[action("Advance to the next column.")]
            |different, &mut column| *column += 1;

            #[action("Produce the matching position.")]
            let found = |equal, &row, &column| Some((**row, *column));

            |found| break found;
        };

        #[question("Was the target found in this row?")]
        let (done, again) = |&found| found.is_some();

        |done, found| break found;

        #[action("Advance to the next row.")]
        |again, &mut row| *row += 1;
    };

    |result| return result;
}

#[test]
fn an_inner_cycle_result_can_finish_the_entire_flow() {
    let rows = [&[][..], &[1, 2][..], &[3][..]];
    assert_eq!(nested_search(&[], 1), None);
    assert_eq!(nested_search(&rows, 3), Some((2, 0)));
    assert_eq!(nested_search(&rows, 2), Some((1, 1)));
    assert_eq!(nested_search(&rows, 4), None);
}
