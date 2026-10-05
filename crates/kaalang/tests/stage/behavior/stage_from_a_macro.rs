use kaalang::kaalang;

macro_rules! staged {
    ($name:ident, $entry:ident, $statement:expr) => {
        #[kaalang]
        fn $name(value: u32) -> u32 {
            #[action("Prepare the entry.")]
            let $entry = |value| value;

            #[stage("Increment the entry.")]
            $statement;
        }
    };
}

macro_rules! stage_body {
    ($name:ident, $entry:ident, $body:block) => {
        staged!($name, $entry, |$entry| $body);
    };
}

stage_body!(stage_from_a_macro, entry, {
    #[action("Add one.")]
    let result = |entry| entry + 1;

    |result| return result;
});

#[test]
fn a_stage_statement_and_body_from_macros_run() {
    assert_eq!(stage_from_a_macro(4), 5);
}
