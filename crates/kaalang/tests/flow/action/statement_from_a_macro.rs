use std::cell::Cell;

use kaalang::kaa;

/// A flow written by another macro, whose whole zero-output action statement
/// arrives wrapped in the invisible group a metavariable substitutes through.
macro_rules! counting {
    ($name:ident, $counter:ident, $statement:expr) => {
        #[kaa]
        fn $name($counter: &Cell<u32>) -> u32 {
            #[kaa::action("Count one visit.")]
            $statement;

            #[kaa::action("Read the count.")]
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
