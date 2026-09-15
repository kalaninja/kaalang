use kaalang::kaalang;

fn origin() -> (i32, i32) {
    (0, 0)
}

/// A flow written by another macro, whose whole application arrives wrapped in
/// the invisible group a metavariable substitutes through.
macro_rules! starting_at {
    ($name:ident, $application:expr) => {
        #[kaalang]
        fn $name() -> (i32, i32) {
            #[call("Start at the origin.")]
            let end = $application;

            |end| return end;
        }
    };
}

starting_at!(call_an_application_from_a_macro, origin());

#[test]
fn a_bare_application_may_arrive_through_a_macro_metavariable() {
    assert_eq!(call_an_application_from_a_macro(), (0, 0));
}
