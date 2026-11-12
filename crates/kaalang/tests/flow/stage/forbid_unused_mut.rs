//! The generated code declares no lint level that a forbidden `unused_mut`
//! would reject.
#![forbid(unused_mut)]

use kaalang::kaa;

#[kaa]
fn forbid_unused_mut(limit: usize) -> usize {
    #[kaa::action("Initialize the counter.")]
    let mut counter = || 0usize;

    #[kaa::action("Begin counting.")]
    let count = || {};

    #[kaa::stage("Count to the limit.")]
    let finish = |count| {
        #[kaa::cycle("Increment until the limit.")]
        let finish = loop {
            #[kaa::question("Done?")]
            let (finish, again) = |&counter, limit| *counter >= limit;

            #[kaa::action("Increment.")]
            let stepped = |again, &mut counter| *counter += 1;

            |stepped| continue;
        };
    };

    #[kaa::stage("Return the count.")]
    |finish| {
        |counter| return counter;
    };
}

#[test]
fn compiles_where_unused_mut_is_forbidden() {
    assert_eq!(forbid_unused_mut(3), 3);
}
