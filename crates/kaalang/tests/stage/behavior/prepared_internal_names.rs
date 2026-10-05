use kaalang::kaalang;

#[kaalang]
fn prepared_internal_names() -> ((), u32) {
    #[action("Prepare a value with an internal-looking name.")]
    let __kaalang_unnamed_value = || 42;

    #[action("Enter the terminal stage.")]
    let __kaalang_unnamed_entry = || {};

    #[stage("Return the prepared value.")]
    |__kaalang_unnamed_entry| {
        |__kaalang_unnamed_entry, __kaalang_unnamed_value| {
            return (__kaalang_unnamed_entry, __kaalang_unnamed_value);
        };
    };
}

#[test]
fn authored_data_and_stage_entries_are_not_synthetic() {
    assert_eq!(prepared_internal_names(), ((), 42));
}
