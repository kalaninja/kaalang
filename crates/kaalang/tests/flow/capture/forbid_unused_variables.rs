//! The generated code declares no lint level that a forbidden lint of an
//! unread capture or of an underscored cycle output would reject.
#![forbid(
    unused_variables,
    clippy::let_unit_value,
    clippy::used_underscore_binding
)]

use kaalang::kaa;

#[kaa]
fn forbid_unused_variables(limit: u32) -> u32 {
    #[kaa::action("Start at zero.")]
    let mut count = || 0;

    #[kaa::cycle("Count to the limit.")]
    let _done = loop {
        #[kaa::question("Is the limit reached?")]
        let (_done, more) = |&count, limit| *count >= limit;

        #[kaa::action("Count one.")]
        let counted = |more, &mut count| *count += 1;

        |counted| continue;
    };

    |_done, count| return count;
}

#[test]
fn compiles_where_an_unread_capture_is_forbidden() {
    assert_eq!(forbid_unused_variables(3), 3);
}
