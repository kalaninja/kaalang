//! Heavy synthetic flows that stress combinations of language features,
//! compiler analysis, and diagram rendering.

use std::cell::Cell;

mod branching_with_work;
mod cyclic_branching_with_work;
mod process_batch;
mod staged_branching_with_work;

thread_local! { static ROUTE: Cell<usize> = const { Cell::new(0) }; }

fn selected(level: usize) -> usize {
    let divisor: usize = [2, 3, 2, 3, 2][..level].iter().product();
    ROUTE.get() / divisor % [2, 3, 2, 3, 2][level]
}

fn expected(seed: usize, config: usize) -> usize {
    let mut values = vec![seed];
    for level in 0..5 {
        let mut next = values[level] + config + selected(level);
        if level > 0 {
            next += values[if level > 1 { level / 2 } else { 0 }];
        }
        values.push(next);
    }
    values[5]
}
