use kaalang::kaalang;

#[kaalang]
fn borrowed_common(condition: bool, prefix: String) -> String {
    #[question("Choose a suffix.")]
    let (yes, no) = |condition| condition;

    #[action("Build the yes suffix.")]
    let suffix = |yes, &prefix| format!("{prefix}-yes");

    #[action("Build the no suffix.")]
    let suffix = |no, &prefix| format!("{prefix}-no");

    #[action("Use the suffix and the preserved prefix.")]
    let end = |suffix, prefix| format!("{prefix}:{suffix}");
}

#[test]
fn borrowed_common_wire_survives_convergence() {
    assert_eq!(borrowed_common(true, "root".into()), "root:root-yes");
    assert_eq!(borrowed_common(false, "root".into()), "root:root-no");
}
