use std::cell::Cell;

use kaalang::kaalang;

/// A flow written by another macro, whose whole zero-output action statement
/// arrives wrapped in the invisible group a metavariable substitutes through.
macro_rules! counting {
    ($name:ident, $counter:ident, $statement:expr) => {
        #[kaalang]
        fn $name($counter: &Cell<u32>) -> u32 {
            #[action("Count one visit.")]
            $statement;

            #[action("Read the count.")]
            let end = |$counter| $counter.get();

            |end| return end;
        }
    };
}

counting!(statement_from_a_macro, counter, |counter| counter
    .set(counter.get() + 1));

#[test]
fn a_zero_output_action_statement_from_a_macro_runs() {
    let counter = Cell::new(0);
    assert_eq!(statement_from_a_macro(&counter), 1);
}
