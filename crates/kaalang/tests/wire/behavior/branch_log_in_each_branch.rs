use kaalang::kaalang;

/// The other correction: each branch transforms the value itself, so the string
/// moves only on the selected path and the two `result` outputs merge before end.
#[kaalang]
fn branch_log_in_each_branch(condition: bool, value: String) -> usize {
    #[question("Log the value?")]
    |condition| -> (yes, no) { condition };

    #[action("Log.")]
    |yes, &value| -> logged {
        assert_eq!(value, "abc");
    };

    #[action("Transform after logging.")]
    |logged, value| -> result { value.len() };

    #[action("Transform without logging.")]
    |no, value| -> result { value.len() };
}

#[test]
fn each_branch_transforms_its_own_value() {
    assert_eq!(branch_log_in_each_branch(true, String::from("abc")), 3);
    assert_eq!(branch_log_in_each_branch(false, String::from("abc")), 3);
}
