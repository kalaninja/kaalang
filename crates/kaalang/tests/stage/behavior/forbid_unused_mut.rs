//! The generated code declares no lint level that a forbidden `unused_mut`
//! would reject.
#![forbid(unused_mut)]

use kaalang::kaalang;

#[kaalang]
fn forbid_unused_mut(limit: usize) -> usize {
    #[action("Initialize the counter.")]
    let mut counter = || 0usize;

    #[action("Begin counting.")]
    let count = || {};

    #[stage("Count to the limit.")]
    let finish = |count| {
        #[cycle("Increment until the limit.")]
        let finish = loop {
            #[question("Done?")]
            let (finish, again) = |&counter, limit| *counter >= limit;

            #[action("Increment.")]
            let stepped = |again, &mut counter| *counter += 1;

            |stepped| continue;
        };
    };

    #[stage("Return the count.")]
    |finish| {
        |counter| return counter;
    };
}

#[test]
fn compiles_where_unused_mut_is_forbidden() {
    assert_eq!(forbid_unused_mut(3), 3);
}
