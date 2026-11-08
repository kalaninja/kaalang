use kaalang::kaa;

#[kaa]
fn for_independent_ranges(rows: u32, columns: u32) -> u32 {
    #[kaa::action("Start with no visited cells.")]
    let mut visited = || 0;

    #[kaa::cycle("Visit every row.")]
    |rows| {
        for _ in 0..rows {
            #[kaa::cycle("Visit every column.")]
            |columns| {
                for _ in 0..columns {
                    #[kaa::action("Count one cell.")]
                    |&mut visited| *visited += 1;
                }
            };
        }
    };

    |visited| return visited;
}

#[test]
fn nested_ranges_need_no_capture_of_the_outer_item() {
    assert_eq!(for_independent_ranges(0, 3), 0);
    assert_eq!(for_independent_ranges(2, 0), 0);
    assert_eq!(for_independent_ranges(2, 3), 6);
}
