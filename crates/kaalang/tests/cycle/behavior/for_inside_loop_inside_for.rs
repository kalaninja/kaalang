use kaalang::kaalang;

#[kaalang]
fn for_inside_loop_inside_for(rows: u32, columns: u32) -> u32 {
    #[action("Start with no visited cells.")]
    let mut visited = || 0;

    #[cycle("Visit every row.")]
    |rows| {
        for _ in 0..rows {
            #[cycle("Complete one row.")]
            let _finished = loop {
                #[cycle("Visit every column.")]
                |columns| {
                    for _ in 0..columns {
                        #[action("Count one cell.")]
                        |&mut visited| *visited += 1;
                    }
                };

                #[action("Finish the row.")]
                let _finished = || {};
            };
        }
    };

    |visited| return visited;
}

#[test]
fn an_empty_outer_range_skips_cycles_nested_through_a_loop() {
    assert_eq!(for_inside_loop_inside_for(0, 3), 0);
    assert_eq!(for_inside_loop_inside_for(2, 0), 0);
    assert_eq!(for_inside_loop_inside_for(2, 3), 6);
}
