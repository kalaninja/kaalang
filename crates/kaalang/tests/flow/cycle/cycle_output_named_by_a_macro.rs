use kaalang::kaa;

macro_rules! counted {
    ($name:ident, $done:ident) => {
        #[kaa]
        fn $name(limit: u32) -> u32 {
            #[kaa::action("Start at zero.")]
            let mut count = || 0;

            #[kaa::cycle("Count to the limit.")]
            let $done = loop {
                #[kaa::question("Is the limit reached?")]
                let ($done, more) = |&count, limit| *count >= limit;

                #[kaa::action("Count one.")]
                let counted = |more, &mut count| *count += 1;

                |counted| continue;
            };

            |$done, count| return count;
        }
    };
}

counted!(cycle_output_named_by_a_macro, _done);

#[test]
fn a_cycle_output_named_at_the_invocation_leaves_the_cycle() {
    assert_eq!(cycle_output_named_by_a_macro(2), 2);
}
