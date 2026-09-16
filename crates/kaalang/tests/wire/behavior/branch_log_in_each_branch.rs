use kaalang::kaalang;

/// Each branch transforms the string on its selected path; the `end` outputs
/// merge before the structural return.
#[kaalang]
fn branch_log_in_each_branch(condition: bool, value: String) -> usize {
    #[question("Log the value?")]
    let (yes, no) = |condition| condition;

    #[action("Log.")]
    let logged = |yes, &value| {
        assert_eq!(value, "abc");
    };

    #[action("Transform after logging.")]
    let end = |logged, value| value.len();

    #[action("Transform without logging.")]
    let end = |no, value| value.len();

    |end| return end;
}

#[test]
fn each_branch_transforms_its_own_value() {
    assert_eq!(branch_log_in_each_branch(true, String::from("abc")), 3);
    assert_eq!(branch_log_in_each_branch(false, String::from("abc")), 3);
}
