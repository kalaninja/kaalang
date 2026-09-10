use kaalang::kaalang;

#[kaalang]
fn nested_search(rows: &[&[i32]], target: i32) -> Option<(usize, usize)> {
    #[action("Start at the first row.")]
    let mut row = || 0;

    #[question("Are there more rows?")]
    #[no("All rows have been searched.")]
    #[yes("Search this row.")]
    while (|row, rows| row < rows.len()) {
        #[action("Start at the first column.")]
        let mut column = || 0;

        #[question("Are there more columns?")]
        while (|column, row, rows| column < rows[row].len()) {
            #[question("Does the value differ from the target?")]
            let (different, equal) = |rows, row, column, target| rows[row][column] != target;

            #[action("Advance to the next column.")]
            |different, &mut column| *column += 1;

            #[action("Return the matching position.")]
            let end = |equal, row, column| Some((row, column));
        }

        #[action("Advance to the next row.")]
        |&mut row| *row += 1;
    }

    #[action("The target is absent.")]
    let end = || None;
}

#[test]
fn inner_end_wire_finishes_the_entire_flow() {
    let rows = [&[][..], &[1, 2][..], &[3][..]];
    assert_eq!(nested_search(&[], 1), None);
    assert_eq!(nested_search(&rows, 3), Some((2, 0)));
    assert_eq!(nested_search(&rows, 2), Some((1, 1)));
    assert_eq!(nested_search(&rows, 4), None);
}
